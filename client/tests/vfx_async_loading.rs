use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use client::vfx::registry::{Priority, VfxRegistry};
use client::vfx::{
    AssetLoadContext, AssetLoader, LoadRequestId, LoadingState, SpawnResult, VfxSpawnParams,
    VfxSystem,
};

const PARTICLE: &str = include_str!("../../assets/vfx/exemples_vfx/particle.json");
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

struct TemporaryEffect(PathBuf);

impl TemporaryEffect {
    fn new(source: &str) -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let index = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "network-alpha-vfx-{}-{time}-{index}.json",
            std::process::id()
        ));
        std::fs::write(&path, source).unwrap();
        Self(path)
    }

    fn context(&self) -> AssetLoadContext {
        let mut context = AssetLoadContext::new(self.0.clone());
        context.ttl = Duration::from_secs(5);
        context
    }
}

impl Drop for TemporaryEffect {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

async fn collect_result(loader: &mut AssetLoader, request: LoadRequestId) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            loader.run_request().await;
            if matches!(
                loader.state(request),
                Some(LoadingState::Loaded(_) | LoadingState::Failed(_))
            ) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("VFX load did not finish");
}

#[test]
fn async_result_is_published_without_rereading_the_file_and_can_spawn() {
    runtime().block_on(async {
        let file = TemporaryEffect::new(PARTICLE);
        let mut loader = AssetLoader::new();
        let mut system = VfxSystem::default();
        let request = loader.request_load(file.context());

        assert!(
            !system
                .registry
                .register_loaded(&mut loader, request, Priority::Normal)
                .unwrap()
        );
        assert!(system.registry.get("impact_embers").is_none());

        collect_result(&mut loader, request).await;
        std::fs::remove_file(&file.0).unwrap();
        assert!(
            system
                .registry
                .register_loaded(&mut loader, request, Priority::Normal)
                .unwrap()
        );
        assert!(loader.state(request).is_none());

        let api = system.api();
        let ticket = api
            .spawn("impact_embers", VfxSpawnParams::default())
            .unwrap();
        system.update(1.0 / 60.0, |_| None);
        assert!(matches!(
            api.take_result(ticket),
            Some(SpawnResult::Spawned(_))
        ));
        assert_eq!(system.extract().sprites.len(), 24);

        let error = system
            .registry
            .register_loaded(&mut loader, request, Priority::Normal)
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(system.registry.get("impact_embers").unwrap().revision, 1);
    });
}

#[test]
fn invalid_async_reloads_preserve_the_published_revision() {
    runtime().block_on(async {
        let mut registry = VfxRegistry::default();
        registry.register_json(PARTICLE, Priority::Normal).unwrap();
        let previous = Arc::clone(registry.get("impact_embers").unwrap());
        let mut loader = AssetLoader::new();

        for (pointer, replacement) in [
            ("/logical/emission/delay_seconds", serde_json::json!(-1.0)),
            (
                "/logical/initialization/lifetime_seconds/min",
                serde_json::json!(5.0),
            ),
            (
                "/logical/evolution/acceleration/0",
                serde_json::json!(1e100),
            ),
            (
                "/visual/appearance/initial_state",
                serde_json::json!("missing"),
            ),
        ] {
            let mut invalid: serde_json::Value = serde_json::from_str(PARTICLE).unwrap();
            *invalid.pointer_mut(pointer).unwrap() = replacement;
            let file = TemporaryEffect::new(&invalid.to_string());
            let request = loader.request_load(file.context());
            collect_result(&mut loader, request).await;

            let error = registry
                .register_loaded(&mut loader, request, Priority::Essential)
                .unwrap_err();
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::InvalidData,
                "{pointer}: {error}"
            );
            assert!(error.to_string().contains(file.0.to_str().unwrap()));
            assert!(Arc::ptr_eq(
                registry.get("impact_embers").unwrap(),
                &previous
            ));
            assert!(loader.state(request).is_none());
        }
    });
}

#[test]
fn async_reload_keeps_active_instances_on_their_previous_definition() {
    runtime().block_on(async {
        let mut system = VfxSystem::default();
        system
            .registry
            .register_json(PARTICLE, Priority::Normal)
            .unwrap();
        let api = system.api();
        let first = api
            .spawn("impact_embers", VfxSpawnParams::default())
            .unwrap();
        system.update(0.0, |_| None);

        let mut changed: serde_json::Value = serde_json::from_str(PARTICLE).unwrap();
        changed["logical"]["emission"]["count"] = 2.into();
        let file = TemporaryEffect::new(&changed.to_string());
        let mut loader = AssetLoader::new();
        let request = loader.request_load(file.context());
        collect_result(&mut loader, request).await;
        assert!(
            system
                .registry
                .register_loaded(&mut loader, request, Priority::Essential)
                .unwrap()
        );
        let definition = system.registry.get("impact_embers").unwrap();
        assert_eq!(definition.revision, 2);
        assert_eq!(definition.priority, Priority::Essential);

        let second = api
            .spawn("impact_embers", VfxSpawnParams::default())
            .unwrap();
        system.update(1.0 / 60.0, |_| None);
        assert!(matches!(
            api.take_result(first),
            Some(SpawnResult::Spawned(_))
        ));
        assert!(matches!(
            api.take_result(second),
            Some(SpawnResult::Spawned(_))
        ));
        assert_eq!(system.extract().sprites.len(), 26);
    });
}

#[test]
fn read_errors_and_cancelled_requests_are_reported_without_publication() {
    runtime().block_on(async {
        let file = TemporaryEffect::new(PARTICLE);
        let mut loader = AssetLoader::new();
        let mut registry = VfxRegistry::default();
        let missing = loader.request_load(AssetLoadContext::new(file.0.join("missing.json")));
        collect_result(&mut loader, missing).await;
        assert!(
            registry
                .register_loaded(&mut loader, missing, Priority::Normal)
                .is_err()
        );

        let cancelled = loader.request_load(file.context());
        assert!(loader.cancel(cancelled));
        let error = registry
            .register_loaded(&mut loader, cancelled, Priority::Normal)
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(registry.get("impact_embers").is_none());
    });
}

#![allow(dead_code)]
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use utils::arena::*;

use crate::vfx::parser::parse_effect;
use crate::vfx::parser::r#struct::VfxEffectDescriptor;

type LoadHandle = JoinHandle<std::io::Result<VfxEffectDescriptor>>;

#[derive(Default)]
pub enum LoadingState {
    #[default]
    NotLoaded,
    Loading,
    Loaded(VfxEffectDescriptor),
    Failed(std::io::Error),
}

pub struct AssetLoadContext {
    pub path: PathBuf,
    pub state: LoadingState,
    /// Délai maximal du chargement, vérifié lors de `run_request`.
    pub ttl: Duration,
}

impl AssetLoadContext {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            state: LoadingState::NotLoaded,
            ttl: Duration::from_millis(200),
        }
    }
}

pub struct Loader {
    context: AssetLoadContext,
    started_at: Option<Instant>,
}

impl Loader {
    pub fn new(context: AssetLoadContext) -> Self {
        Self {
            context,
            started_at: None,
        }
    }

    /// Doit être appelé dans un runtime Tokio actif.
    pub fn load(&mut self) -> LoadHandle {
        let path = self.context.path.clone();
        self.started_at = Some(Instant::now());
        self.context.state = LoadingState::Loading;
        // Lecture et parsing restent hors de la boucle qui collecte les résultats.
        tokio::task::spawn_blocking(move || {
            let content = std::fs::read_to_string(path)?;
            parse_effect(&content)
        })
    }
}

#[derive(Default)]
pub struct AssetLoader {
    loaders: Arena<Loader>,
    future_handles: Vec<(Id<Loader>, LoadHandle)>,
    finished_loads: Vec<usize>,
}

impl AssetLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Doit être appelé dans un runtime Tokio actif.
    pub fn request_load(&mut self, context: AssetLoadContext) -> Id<Loader> {
        let mut loader = Loader::new(context);
        let handle = loader.load();
        let id = self.loaders.insert(loader);
        self.future_handles.push((id, handle));
        id
    }

    pub fn state(&self, id: Id<Loader>) -> Option<&LoadingState> {
        self.loaders.get(id).map(|loader| &loader.context.state)
    }

    /// Une récupération prématurée conserve la requête et son résultat futur.
    pub fn take_loaded_asset(&mut self, id: Id<Loader>) -> Option<VfxEffectDescriptor> {
        if !matches!(self.state(id), Some(LoadingState::Loaded(_))) {
            return None;
        }
        match self.loaders.remove(id)?.context.state {
            LoadingState::Loaded(descriptor) => Some(descriptor),
            _ => None,
        }
    }

    pub fn take_error(&mut self, id: Id<Loader>) -> Option<std::io::Error> {
        if !matches!(self.state(id), Some(LoadingState::Failed(_))) {
            return None;
        }
        match self.loaders.remove(id)?.context.state {
            LoadingState::Failed(error) => Some(error),
            _ => None,
        }
    }

    /// Libère la requête. Une lecture/parsing déjà démarré peut finir en arrière-plan.
    pub fn cancel(&mut self, id: Id<Loader>) -> bool {
        if self.loaders.get(id).is_none() {
            return false;
        }
        if let Some(index) = self.future_handles.iter().position(|(key, _)| *key == id) {
            self.future_handles.swap_remove(index).1.abort();
        }
        self.loaders.remove(id).is_some()
    }

    pub async fn run_request(&mut self) {
        self.check_finished_loads();
        self.drain_finished_loads().await;
    }

    fn check_finished_loads(&mut self) {
        self.finished_loads.clear();
        for (index, (id, handle)) in self.future_handles.iter().enumerate() {
            let expired = self.loaders.get(*id).is_some_and(|loader| {
                loader
                    .started_at
                    .is_some_and(|start| start.elapsed() >= loader.context.ttl)
            });
            if handle.is_finished() || expired {
                self.finished_loads.push(index);
            }
        }
    }

    async fn drain_finished_loads(&mut self) {
        // Les suppressions ne doivent pas invalider les indices restant à traiter.
        for index in self.finished_loads.drain(..).rev() {
            let (id, handle) = self.future_handles.swap_remove(index);
            let Some(loader) = self.loaders.get_mut(id) else {
                handle.abort();
                continue;
            };
            if !handle.is_finished() {
                handle.abort();
                loader.context.state = LoadingState::Failed(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "asset loading timed out",
                ));
                continue;
            }
            loader.context.state = match handle.await {
                Ok(Ok(descriptor)) => LoadingState::Loaded(descriptor),
                Ok(Err(error)) => LoadingState::Failed(error),
                Err(error) => LoadingState::Failed(std::io::Error::other(error)),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const PARTICLE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/config/vfx/exemples_vfx/particle.json"
    );

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    async fn settle(loader: &mut AssetLoader) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !loader.future_handles.is_empty() {
                loader.run_request().await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("asset loading timed out");
    }

    #[test]
    fn check_workflow_and_early_take() {
        runtime().block_on(async {
            let mut loader = AssetLoader::new();
            let mut context = AssetLoadContext::new(PathBuf::from(PARTICLE_PATH));
            context.ttl = Duration::from_secs(5);
            let id = loader.request_load(context);
            assert!(loader.take_loaded_asset(id).is_none());
            assert!(matches!(loader.state(id), Some(LoadingState::Loading)));
            settle(&mut loader).await;
            let expected = parse_effect(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/config/vfx/exemples_vfx/particle.json"
            )))
            .unwrap();
            assert_eq!(loader.take_loaded_asset(id).unwrap(), expected);
            assert!(loader.take_loaded_asset(id).is_none());
            assert!(loader.state(id).is_none());
        });
    }

    #[test]
    fn collects_multiple_finished_loads() {
        runtime().block_on(async {
            let mut loader = AssetLoader::new();
            let mut ids = Vec::new();
            for _ in 0..3 {
                let id = loader
                    .loaders
                    .insert(Loader::new(AssetLoadContext::new(PARTICLE_PATH)));
                let handle = tokio::spawn(async {
                    parse_effect(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../assets/config/vfx/exemples_vfx/particle.json"
                    )))
                });
                ids.push(id);
                loader.future_handles.push((id, handle));
            }
            while !loader
                .future_handles
                .iter()
                .all(|(_, handle)| handle.is_finished())
            {
                tokio::task::yield_now().await;
            }
            loader.run_request().await;
            assert!(loader.future_handles.is_empty());
            for id in ids {
                assert!(loader.take_loaded_asset(id).is_some());
            }
        });
    }

    #[test]
    fn reports_read_and_parse_errors() {
        runtime().block_on(async {
            let mut loader = AssetLoader::new();
            let missing = loader.request_load(AssetLoadContext::new(
                PathBuf::from(PARTICLE_PATH).join("missing.json"),
            ));
            // Cargo.toml existe mais n'est pas un descripteur JSON.
            let invalid = loader.request_load(AssetLoadContext::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/Cargo.toml"
            )));
            settle(&mut loader).await;
            for id in [missing, invalid] {
                assert!(matches!(loader.state(id), Some(LoadingState::Failed(_))));
                assert!(loader.take_loaded_asset(id).is_none());
                assert!(loader.take_error(id).is_some());
                assert!(loader.take_error(id).is_none());
            }
        });
    }

    #[test]
    fn timeout_and_cancel_preserve_other_requests() {
        runtime().block_on(async {
            let mut loader = AssetLoader::new();
            let mut context = AssetLoadContext::new(PARTICLE_PATH);
            context.ttl = Duration::ZERO;
            let mut entry = Loader::new(context);
            entry.started_at = Some(Instant::now());
            entry.context.state = LoadingState::Loading;
            let id = loader.loaders.insert(entry);
            loader
                .future_handles
                .push((id, tokio::spawn(std::future::pending())));
            loader.run_request().await;
            assert_eq!(
                loader.take_error(id).unwrap().kind(),
                std::io::ErrorKind::TimedOut
            );
            let cancelled = loader.request_load(AssetLoadContext::new(PARTICLE_PATH));
            assert!(loader.cancel(cancelled));
            assert!(!loader.cancel(cancelled));
            let next = loader.request_load(AssetLoadContext::new(PARTICLE_PATH));
            assert!(loader.take_loaded_asset(cancelled).is_none());
            settle(&mut loader).await;
            assert!(loader.take_loaded_asset(next).is_some());
        });
    }
}

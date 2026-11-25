//! Name-based async loading, shared dependencies and child-before-parent publication.
use crate::vfx::document::VfxDocument;
use crate::vfx::{
    AssetLoadContext, AssetLoader, LoadRequestId,
    registry::{Priority, VfxRegistry},
};
use std::{
    collections::{HashMap, HashSet},
    path::{Component, PathBuf},
    time::Duration,
};

enum Load {
    Reading(LoadRequestId),
    Parsed(VfxDocument),
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("alpha-vfx-{}", rand::random::<u64>()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn write(&self, name: &str, source: &str) {
            std::fs::write(self.0.join(name), source).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for name in ["a.json", "b.json"] {
                let _ = std::fs::remove_file(self.0.join(name));
            }
            let _ = std::fs::remove_dir(&self.0);
        }
    }

    async fn pump_until(
        assets: &mut VisualAssets,
        loader: &mut AssetLoader,
        registry: &mut VfxRegistry,
        condition: impl Fn(&VisualAssets, &VfxRegistry) -> bool,
    ) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !condition(assets, registry) {
            assert!(std::time::Instant::now() < deadline);
            loader.run_request().await;
            assets.publish(loader, registry);
            tokio::task::yield_now().await;
        }
    }

    #[test]
    fn failed_dependency_can_retry_without_publishing_an_incomplete_composite() {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
            let fixture = Fixture::new();
            fixture.write("a.json", r#"{"version":1,"name":"a","family":"composite","components":[{"effect":"b","delay_seconds":0}]}"#);
            let mut assets = VisualAssets::new(fixture.0.clone(), r#"{"a":"a.json","b":"b.json"}"#).unwrap();
            let mut loader = AssetLoader::new();
            let mut registry = VfxRegistry::default();
            assets.ensure_loaded("a");
            assets.ensure_loaded("a");
            pump_until(&mut assets, &mut loader, &mut registry, |assets, _| assets.error("a").is_some()).await;
            assert!(registry.get("a").is_none());
            assert!(matches!(assets.loads.get("b"), Some(Load::Failed(_))));
            fixture.write("b.json", r#"{"version":1,"name":"b","family":"feedback","feedback":{"type":"camera_shake","trauma":0.1}}"#);
            assets.retry_failed();
            pump_until(&mut assets, &mut loader, &mut registry, |_, registry| registry.get("a").is_some()).await;
            assert_eq!(registry.get("a").unwrap().revision, 1);
            assert_eq!(registry.get("b").unwrap().revision, 1);
            assert!(assets.loads.is_empty());
        });
    }

    #[test]
    fn dependency_cycles_fail_without_hanging_or_publishing() {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
            let fixture = Fixture::new();
            fixture.write("a.json", r#"{"version":1,"name":"a","family":"composite","components":[{"effect":"b","delay_seconds":0}]}"#);
            fixture.write("b.json", r#"{"version":1,"name":"b","family":"composite","components":[{"effect":"a","delay_seconds":0}]}"#);
            let mut assets = VisualAssets::new(fixture.0.clone(), r#"{"a":"a.json","b":"b.json"}"#).unwrap();
            let mut loader = AssetLoader::new();
            let mut registry = VfxRegistry::default();
            assets.ensure_loaded("a");
            pump_until(&mut assets, &mut loader, &mut registry, |assets, _| assets.error("a").is_some()).await;
            assert!(assets.error("a").unwrap().contains("cycle"));
            assert!(registry.get("a").is_none());
            assert!(registry.get("b").is_none());
        });
    }
}

pub struct VisualAssets {
    root: PathBuf,
    catalog: HashMap<String, String>,
    wanted: HashSet<String>,
    loads: HashMap<String, Load>,
}

impl VisualAssets {
    pub fn new(root: PathBuf, catalog: &str) -> std::io::Result<Self> {
        let catalog: HashMap<String, String> = serde_json::from_str(catalog)?;
        for path in catalog.values() {
            if path.is_empty()
                || std::path::Path::new(path)
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(std::io::Error::other(
                    "VFX catalog paths must be relative to its root",
                ));
            }
        }
        Ok(Self {
            root,
            catalog,
            wanted: HashSet::new(),
            loads: HashMap::new(),
        })
    }

    pub fn bundled() -> std::io::Result<Self> {
        Self::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/vfx"),
            include_str!("../../../assets/config/vfx_catalog.json"),
        )
    }

    pub fn ensure_loaded(&mut self, effect: &str) {
        self.wanted.insert(effect.to_owned());
    }

    pub fn error(&self, effect: &str) -> Option<&str> {
        match self.loads.get(effect) {
            Some(Load::Failed(error)) => Some(error),
            _ => None,
        }
    }

    /// Explicit retry avoids restarting broken assets every frame.
    pub fn retry_failed(&mut self) {
        self.loads
            .retain(|_, load| !matches!(load, Load::Failed(_)));
    }

    /// Run inside Tokio, after AssetLoader::run_request. Never waits for file IO.
    pub fn publish(&mut self, loader: &mut AssetLoader, registry: &mut VfxRegistry) {
        let names: Vec<_> = self.loads.keys().cloned().collect();
        for name in names {
            let Some(Load::Reading(ticket)) = self.loads.get(&name) else {
                continue;
            };
            let ticket = *ticket;
            if let Some(error) = loader.take_error(ticket) {
                self.fail(&name, error.to_string());
            } else if let Some(document) = loader.take_document(ticket) {
                if document.name() != name {
                    self.fail(
                        &name,
                        format!("catalog name {name} differs from {}", document.name()),
                    );
                } else {
                    self.loads.insert(name, Load::Parsed(document));
                }
            }
        }
        let wanted: Vec<_> = self.wanted.iter().cloned().collect();
        for name in wanted {
            let _ = self.resolve(&name, &mut Vec::new(), loader, registry);
        }
    }

    fn fail(&mut self, name: &str, error: String) {
        tracing::warn!(effect = name, %error, "VFX loading failed");
        self.loads.insert(name.to_owned(), Load::Failed(error));
    }

    fn resolve(
        &mut self,
        name: &str,
        stack: &mut Vec<String>,
        loader: &mut AssetLoader,
        registry: &mut VfxRegistry,
    ) -> Result<bool, String> {
        if registry.get(name).is_some() {
            return Ok(true);
        }
        if stack.len() > 8 || stack.iter().any(|parent| parent == name) {
            return Err(format!(
                "VFX dependency cycle or depth exceeded: {stack:?} -> {name}"
            ));
        }
        if let Some(Load::Failed(error)) = self.loads.get(name) {
            return Err(error.clone());
        }
        if !self.loads.contains_key(name) {
            let Some(path) = self.catalog.get(name) else {
                let error = format!("unknown VFX catalog entry: {name}");
                self.fail(name, error.clone());
                return Err(error);
            };
            let mut context = AssetLoadContext::new(self.root.join(path));
            context.ttl = Duration::from_secs(5);
            self.loads
                .insert(name.to_owned(), Load::Reading(loader.request_load(context)));
            return Ok(false);
        }
        let Some(Load::Parsed(document)) = self.loads.get(name) else {
            return Ok(false);
        };
        let dependencies = document.dependencies();
        stack.push(name.to_owned());
        let mut ready = true;
        for child in dependencies {
            match self.resolve(&child, stack, loader, registry) {
                Ok(child_ready) => ready &= child_ready,
                Err(error) => {
                    stack.pop();
                    self.fail(name, error.clone());
                    return Err(error);
                }
            }
        }
        stack.pop();
        if !ready {
            return Ok(false);
        }
        let Some(Load::Parsed(document)) = self.loads.remove(name) else {
            unreachable!()
        };
        if let Err(error) = registry.register_document(document, Priority::Normal) {
            self.fail(name, error.to_string());
            return Err(error.to_string());
        }
        Ok(true)
    }
}

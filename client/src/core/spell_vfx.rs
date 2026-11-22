use crate::vfx::{
    AssetLoadContext, AssetLoader, LoadRequestId,
    registry::{Priority, VfxRegistry},
};

/// Chargements conservés jusqu'à publication du composite et de ses composants.
#[derive(Default)]
pub struct SpellVfxLoads {
    fireball: Vec<LoadRequestId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfx::{SpawnResult, VfxSpawnParams, VfxSystem};

    #[test]
    fn acquisition_loads_the_complete_fireball_once_and_it_can_spawn() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut loader = AssetLoader::new();
            let mut system = VfxSystem::default();
            let mut loads = SpellVfxLoads::default();
            loads.request_fireball(&mut loader, &system.registry);
            loads.request_fireball(&mut loader, &system.registry);
            assert_eq!(loads.fireball.len(), 4);
            assert!(system.registry.get("fireball").is_none());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while system.registry.get("fireball").is_none() {
                assert!(std::time::Instant::now() < deadline);
                loader.run_request().await;
                loads.publish(&mut loader, &mut system.registry).unwrap();
                tokio::task::yield_now().await;
            }
            for name in [
                "fireball_core",
                "fireball_flames",
                "fireball_trail",
                "fireball_light",
            ] {
                assert!(system.registry.get(name).is_some());
            }
            loads.request_fireball(&mut loader, &system.registry);
            assert!(loads.fireball.is_empty());
            assert_eq!(system.registry.get("fireball").unwrap().revision, 1);
            let ticket = system
                .api()
                .spawn("fireball", VfxSpawnParams::default())
                .unwrap();
            system.update(1.0 / 60.0, |_| None);
            assert!(matches!(
                system.api().take_result(ticket),
                Some(SpawnResult::Spawned(_))
            ));
        });
    }

    #[test]
    fn failed_load_is_consumed_without_publishing_a_composite_and_can_retry() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut loader = AssetLoader::new();
            let mut registry = VfxRegistry::default();
            let missing =
                std::env::temp_dir().join(format!("missing-vfx-{}.json", rand::random::<u64>()));
            let request = loader.request_load(AssetLoadContext::new(missing));
            let mut loads = SpellVfxLoads {
                fireball: vec![request],
            };
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                assert!(std::time::Instant::now() < deadline);
                loader.run_request().await;
                if loads.publish(&mut loader, &mut registry).is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
            assert!(loads.fireball.is_empty());
            assert!(loader.state(request).is_none());
            assert!(registry.get("fireball").is_none());
            loads.request_fireball(&mut loader, &registry);
            assert_eq!(loads.fireball.len(), 4);
            for request in loads.fireball.drain(..) {
                loader.cancel(request);
            }
        });
    }
}

impl SpellVfxLoads {
    /// À appeler dans le runtime Tokio lors de l'acquisition d'un sort.
    pub fn request_fireball(&mut self, loader: &mut AssetLoader, registry: &VfxRegistry) {
        if registry.get("fireball").is_some() || !self.fireball.is_empty() {
            return;
        }
        for file in ["core.json", "flames.json", "trail.json", "light.json"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../assets/vfx/fireball")
                .join(file);
            self.fireball
                .push(loader.request_load(AssetLoadContext::new(path)));
        }
    }

    /// Collecte non bloquante, après `AssetLoader::run_request` à chaque frame.
    pub fn publish(
        &mut self,
        loader: &mut AssetLoader,
        registry: &mut VfxRegistry,
    ) -> std::io::Result<()> {
        if self.fireball.is_empty() {
            return Ok(());
        }
        let mut index = 0;
        while index < self.fireball.len() {
            match registry.register_loaded(loader, self.fireball[index], Priority::Normal) {
                Ok(true) => {
                    self.fireball.swap_remove(index);
                }
                Ok(false) => index += 1,
                Err(error) => {
                    for request in self.fireball.drain(..) {
                        loader.cancel(request);
                    }
                    return Err(error);
                }
            }
        }
        if self.fireball.is_empty() {
            registry.register_json(
                include_str!("../../../assets/vfx/fireball.json"),
                Priority::Normal,
            )?;
        }
        Ok(())
    }
}

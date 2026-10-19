#![allow(dead_code)]
use tokio::task::JoinHandle;
use utils::arena::*;

use crate::vfx::parser::parse_effect;
use crate::vfx::parser::r#struct::VfxEffectDescriptor;

// TODO: A regarder les vidéos 145 et 146 de Hand made hero

#[derive(Default)]
pub enum LoadingState {
    #[default]
    NotLoaded,
    Loading,
    Loaded(VfxEffectDescriptor),
    Failed(tokio::io::Error), // Il faudrait que je fasse en sorte que la version possède un erreur comme champs
}

pub struct AssetLoadContext {
    pub path: &'static str,
    pub state: LoadingState,
    pub ttl: f32, // Temps de vie de la requête en ms
}

impl AssetLoadContext {
    pub fn new(path: &'static str) -> Self {
        AssetLoadContext {
            path,
            state: LoadingState::NotLoaded,
            ttl: 200.0,
        }
    }
}
pub struct Loader {
    id: Id<Self>,
    context: AssetLoadContext,
}

impl Loader {
    pub fn new(context: AssetLoadContext) -> Self {
        Loader {
            id: Id::zero(),
            context,
        }
    }

    pub fn load(&mut self) -> JoinHandle<Result<String, tokio::io::Error>> {
        let path = self.context.path.to_string();
        let handle = tokio::spawn(tokio::fs::read_to_string(path));
        self.context.state = LoadingState::Loading;
        handle
    }
}

#[derive(Default)]
pub struct AssetLoader {
    loaders: Arena<Loader>,
    future_handles: Vec<(Id<Loader>, JoinHandle<tokio::io::Result<String>>)>,
    finished_loads: Vec<usize>,
}

impl AssetLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_load(&mut self, context: AssetLoadContext) -> Id<Loader> {
        let loader_id = self.loaders.insert(Loader::new(context));
        let loader = self.loaders.get_mut(loader_id).unwrap();
        loader.id = loader_id;

        self.future_handles.push((loader_id, loader.load()));
        loader_id
    }

    pub fn take_loaded_asset(&mut self, id: Id<Loader>) -> Option<VfxEffectDescriptor> {
        let loader = self.loaders.remove(id).unwrap();

        match loader.context.state {
            LoadingState::Loaded(d) => return Some(d),
            _ => return None,
        }
    }

    pub async fn run_request(&mut self) {
        self.check_finished_loads();
        self.drain_finished_loads().await;
    }

    fn push_handle(&mut self, handle: (Id<Loader>, JoinHandle<tokio::io::Result<String>>)) {
        self.future_handles.push(handle);
    }

    fn check_finished_loads(&mut self) {
        for (i, (_, handle)) in self.future_handles.iter().enumerate() {
            if handle.is_finished() {
                self.finished_loads.push(i);
            }
        }
    }

    async fn drain_finished_loads(&mut self) {
        for handle_index in self.finished_loads.drain(..) {
            let (id, handle) = self.future_handles.remove(handle_index);

            let loader = self.loaders.get_mut(id).unwrap();
            match handle.await {
                Ok(Ok(content)) => {
                    let descriptor = parse_effect(&content).unwrap();
                    loader.context.state = LoadingState::Loaded(descriptor);
                }
                Ok(Err(e)) => {
                    loader.context.state = LoadingState::Failed(e.into());
                }
                Err(e) => {
                    loader.context.state = LoadingState::Failed(e.into());
                }
            }
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

    fn expected_result() -> VfxEffectDescriptor {
        let content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/config/vfx/exemples_vfx/particle.json"
        ));
        parse_effect(&content.to_owned()).unwrap()
    }

    #[test]
    fn check_workflow() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut asset_loader = AssetLoader::new();
            let context = AssetLoadContext::new(PARTICLE_PATH);
            let load_id = asset_loader.request_load(context);

            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    asset_loader.run_request().await;
                    match &asset_loader.loaders.get(load_id).unwrap().context.state {
                        LoadingState::Loaded(_) => break,
                        LoadingState::Failed(error) => panic!("loading failed: {error}"),
                        _ => tokio::task::yield_now().await,
                    }
                }
            })
            .await
            .expect("asset loading timed out");

            let descriptor = asset_loader.take_loaded_asset(load_id).unwrap();
            assert_eq!(descriptor, expected_result());
        });
    }
}

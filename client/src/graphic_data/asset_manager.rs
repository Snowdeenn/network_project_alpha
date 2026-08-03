
use utils::ids::{TextureId};

use crate::graphic_data::animation_manager::AnimationManager;
pub struct AssetManager {
    textures: prism::TextureManager,
    anims: AnimationManager,
}

#[allow(dead_code)]
impl AssetManager {
    pub fn new() -> Self {
        Self {
            textures: prism::TextureManager::new(),
            anims: AnimationManager::new(),
        }
    }

    /// Charge une texture unique et retourne son TextureId générationnel
    pub fn load_texture(
        &mut self,
        ctx: &GpuContext,
        path: &str,
    ) -> Option<TextureId> {
        self.textures.load(ctx, path)
    }

    /// Charge le fichier de configuration JSON et enregistre toutes les animations
    pub fn load_animations(
        &mut self,
        ctx: &prism::GpuContext,
        config_path: &str,
    ) {
        self.anims.load_from_config(ctx, &mut self.textures, config_path);
    }

    pub fn textures(&self) -> &prism::TextureManager {
        &self.textures
    }

    pub fn textures_mut(&mut self) -> &mut prism::TextureManager {
        &mut self.textures
    }

    pub fn anims(&self) -> &AnimationManager {
        &self.anims
    }

    pub fn anims_mut(&mut self) -> &mut AnimationManager {
        &mut self.anims
    }

    pub fn split_mut(&mut self) -> (&mut prism::TextureManager, &mut AnimationManager) {
        (&mut self.textures, &mut self.anims)
    }
}
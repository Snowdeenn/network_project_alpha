use crate::graphic_data::animation_manager::{AnimationError, AnimationManager};

pub struct AssetManager {
    anims: AnimationManager,
}

#[allow(dead_code)]
impl AssetManager {
    pub fn new() -> Self {
        Self {
            anims: AnimationManager::new(),
        }
    }
    /// Charge le fichier de configuration JSON et enregistre toutes les animations
    pub fn load_animations(
        &mut self,
        ctx: &prism::GpuContext,
        textures: &mut prism::TextureManager,
        config_path: &str,
    ) -> Result<(), AnimationError> {
        self.anims.load_from_config(ctx, textures, config_path)?;
        Ok(())
    }

    pub fn anims(&self) -> &AnimationManager {
        &self.anims
    }

    pub fn anims_mut(&mut self) -> &mut AnimationManager {
        &mut self.anims
    }

}
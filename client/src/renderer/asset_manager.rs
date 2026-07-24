    // src/renderer/asset_manager.rs

use raylib::prelude::{RaylibHandle, RaylibThread};
use shared::ids::{TextureId};

use crate::renderer::animation_manager::AnimationManager;
use crate::renderer::texture_manager::TextureManager;

pub struct AssetManager {
    textures: TextureManager,
    anims: AnimationManager,
}

#[allow(dead_code)]
impl AssetManager {
    pub fn new() -> Self {
        Self {
            textures: TextureManager::new(),
            anims: AnimationManager::new(),
        }
    }

    // =========================================================================
    // Façade de chargement (Coordonne l'ensemble des sous-managers)
    // =========================================================================

    /// Charge une texture unique et retourne son TextureId générationnel
    pub fn load_texture(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        path: &str,
    ) -> Option<TextureId> {
        self.textures.load(rl, thread, path)
    }

    /// Charge le fichier de configuration JSON et enregistre toutes les animations
    pub fn load_animations(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        config_path: &str,
    ) {
        self.anims.load_from_config(&mut self.textures, rl, thread, config_path);
    }

    // =========================================================================
    // Emprunts disjoints (Borrows séparés pour les sous-systèmes)
    // =========================================================================

    pub fn textures(&self) -> &TextureManager {
        &self.textures
    }

    pub fn textures_mut(&mut self) -> &mut TextureManager {
        &mut self.textures
    }

    pub fn anims(&self) -> &AnimationManager {
        &self.anims
    }

    pub fn anims_mut(&mut self) -> &mut AnimationManager {
        &mut self.anims
    }

    pub fn split_mut(&mut self) -> (&mut TextureManager, &mut AnimationManager) {
        (&mut self.textures, &mut self.anims)
    }
}
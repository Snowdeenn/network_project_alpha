use raylib::prelude::{RaylibHandle, RaylibThread, Texture2D};
use shared::arena::Arena;
use shared::ids::{TextureId, TextureTag};
use ui::provider::TextureProvider;

pub struct TextureManager {
    textures: Arena<Texture2D, TextureTag>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            textures: Arena::new(),
        }
    }

    pub fn load(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        path: &str,
    ) -> Option<TextureId> {
        let texture = rl.load_texture(thread, path).ok()?;
        Some(self.textures.insert(texture))
    }

    pub fn register(&mut self, texture: Texture2D) -> TextureId {
        self.textures.insert(texture)
    }

    pub fn get(&self, id: TextureId) -> Option<&Texture2D> {
        self.textures.get(id)
    }

    pub fn get_mut(&mut self, id: TextureId) -> Option<&mut Texture2D> {
        self.textures.get_mut(id)
    }

    pub fn remove(&mut self, id: TextureId) -> Option<Texture2D> {
        self.textures.remove(id)
    }
}

impl TextureProvider for TextureManager {
    fn get_texture(&self, id: TextureId) -> Option<&Texture2D> {
        self.textures.get(id)
    }
    fn get_texture_mut(&mut self, id: TextureId) -> Option<&mut Texture2D> {
        self.textures.get_mut(id)
    }
}

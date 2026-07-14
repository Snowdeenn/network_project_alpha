use raylib::prelude::{Shader, Texture2D};
use shared::ids::{ShaderId, TextureId};

pub trait ShaderProvider {
    fn get_shader(&self, id: ShaderId) -> Option<&Shader>;
    fn get_shader_mut(&mut self, id: ShaderId) -> Option<&mut Shader>;
}

pub trait TextureProvider {
    fn get_texture(&self, id: TextureId) -> Option<&Texture2D>;
    fn get_texture_mut(&mut self, id: TextureId) -> Option<&mut Texture2D>;
}
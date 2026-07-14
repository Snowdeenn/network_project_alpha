use shared::arena::Arena;
use shared::ids::{ShaderId, ShaderTag};
use raylib::prelude::Shader;
use raylib::shaders::RaylibShader;
use ui::provider::ShaderProvider;


pub struct ShaderManager {
    shaders: Arena<Shader, ShaderTag>,
    elapsed: f32,
}

impl ShaderManager {
    pub fn new() -> Self {
        Self {
            shaders: Arena::new(),
            elapsed: 0.0f32,
        }
    }

    pub fn register(&mut self, shader: Shader) -> ShaderId {
        self.shaders.insert(shader)
    }

    pub fn get(&self, id: ShaderId) -> Option<&Shader> {
        self.shaders.get(id)
    }

    pub fn get_mut(&mut self, id: ShaderId) -> Option<&mut Shader> {
        self.shaders.get_mut(id)
    }

    pub fn remove(&mut self, id: ShaderId) -> Option<Shader> {
        self.shaders.remove(id)
    }

    pub fn swap_shader(&mut self, id: ShaderId, new_shader: Shader) {
        if let Some(slot) = self.shaders.get_mut(id) {
            *slot = new_shader;
        }
    }

    pub fn set_uniform_f32(&mut self, id: ShaderId, name: &str, value: f32) {
        if let Some(shader) = self.shaders.get_mut(id) {
            let loc = shader.get_shader_location(name);
            shader.set_shader_value(loc, value);
        }
    }

    pub fn set_uniform_vec2(&mut self, id: ShaderId, name: &str, value: raylib::math::Vector2) {
        if let Some(shader) = self.shaders.get_mut(id) {
            let loc = shader.get_shader_location(name);
            shader.set_shader_value(loc, value);
        }
    }

    pub fn update_globals(&mut self, dt: f32, screen_w: f32, screen_h: f32) {
        self.elapsed += dt;
        for shader in self.shaders.iter_mut() {
            let loc_time = shader.get_shader_location("u_time");
            let loc_res = shader.get_shader_location("u_resolution");
            if loc_time >= 0 {
                shader.set_shader_value(loc_time, self.elapsed);
            }
            if loc_res >= 0 {
                shader.set_shader_value(loc_res, [screen_w, screen_h]);
            }
        }
    }
}

impl ShaderProvider for ShaderManager {
    fn get_shader(&self, id: ShaderId) -> Option<&Shader> {
       self.shaders.get(id)
    }
    fn get_shader_mut(&mut self, id: ShaderId) -> Option<&mut Shader> {
        self.shaders.get_mut(id)
    }
}

// src/renderer/shader_manager.rs

use raylib::prelude::*;
use utils::arena::Arena;
use utils::ids::{ShaderId, ShaderTag};
use std::collections::HashMap;
use ui::provider::ShaderProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    World,
    Vfx,
    Hud,
    PostProcess,
}

pub struct ShaderManager {
    shaders: Arena<Shader, ShaderTag>,
    pass_shaders: HashMap<PassKind, ShaderId>,
    elapsed: f32,
}

#[allow(dead_code)]
impl ShaderManager {
    pub fn new() -> Self {
        Self {
            shaders: Arena::new(),
            pass_shaders: HashMap::new(),
            elapsed: 0.0,
        }
    }

    pub fn register(&mut self, shader: Shader) -> ShaderId {
        self.shaders.insert(shader)
    }

    pub fn set_pass_shader(&mut self, pass: PassKind, id: ShaderId) {
        self.pass_shaders.insert(pass, id);
    }

    pub fn get_pass_shader(&self, pass: PassKind) -> Option<&Shader> {
        let id = self.pass_shaders.get(&pass)?;
        self.shaders.get(*id)
    }

    pub fn get_pass_shader_mut(&mut self, pass: PassKind) -> Option<&mut Shader> {
        let id = *self.pass_shaders.get(&pass)?;
        self.shaders.get_mut(id)
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

    // --- Uniforms par Batch ---
    pub fn set_uniform_f32(&mut self, id: ShaderId, name: &str, value: f32) {
        if let Some(shader) = self.shaders.get_mut(id) {
            let loc = shader.get_shader_location(name);
            if loc >= 0 {
                shader.set_shader_value(loc, value);
            }
        }
    }

    pub fn set_uniform_vec2(&mut self, id: ShaderId, name: &str, value: Vector2) {
        if let Some(shader) = self.shaders.get_mut(id) {
            let loc = shader.get_shader_location(name);
            if loc >= 0 {
                shader.set_shader_value(loc, value);
            }
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
use utils::arena::Arena;
use utils::ids::{ShaderId, ShaderTag};

use crate::context::GpuContext;

pub struct GpuShader {
    pub module: wgpu::ShaderModule,
    pub path: Option<String>, // None si créé depuis du code inline
}

pub struct ShaderManager {
    shaders: Arena<GpuShader, ShaderTag>,
}

impl ShaderManager {
    pub fn new() -> Self {
        Self {
            shaders: Arena::new(),
        }
    }

    pub fn load(&mut self, ctx: &GpuContext, path: &str) -> Option<ShaderId> {
        let shader_code = std::fs::read_to_string(path).ok()?;

        let module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(path),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        Some(self.shaders.insert(GpuShader {
            module,
            path: Some(path.to_string()),
        }))
    }

    pub fn load_inline(&mut self, ctx: &GpuContext, source: &str, label: &str) -> ShaderId {
        let module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        self.shaders.insert(GpuShader { module, path: None })
    }

    pub fn reload(&mut self, ctx: &GpuContext, id: ShaderId) -> bool {
        let path = match self.shaders.get(id) {
            Some(s) => s.path.clone(),
            None => return false,
        };

        if let Some(path) = path {
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => return false,
            };
            let module = ctx
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&path),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            if let Some(shader) = self.shaders.get_mut(id) {
                shader.module = module;
                return true;
            }
        }
        false
    }

    pub fn get(&self, id: ShaderId) -> Option<&GpuShader> {
        self.shaders.get(id)
    }

    pub fn get_mut(&mut self, id: ShaderId) -> Option<&mut GpuShader> {
        self.shaders.get_mut(id)
    }

    pub fn remove(&mut self, id: ShaderId) -> Option<GpuShader> {
        self.shaders.remove(id)
    }
}

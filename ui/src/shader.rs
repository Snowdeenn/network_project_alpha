use raylib::prelude::Shader;
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderId(u16);

impl ShaderId {
    pub fn value(&self) -> u16 {
        self.0
    }
}

pub struct ShaderRegistry {
    shaders: HashMap<ShaderId, Shader>,
    next_id: u16,
}

impl ShaderRegistry {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn register(&mut self, shader: Shader) -> ShaderId {
        debug_assert!(
            self.next_id < u16::MAX,
            "ShaderRegistry has reached its maximum capacity of shaders."
        );
        let id = ShaderId(self.next_id);
        self.shaders.insert(id, shader);
        self.next_id += 1;
        id
    }

    pub fn get(&self, id: ShaderId) -> Option<&Shader> {
        self.shaders.get(&id)
    }

    pub fn get_mut(&mut self, id: ShaderId) -> Option<&mut Shader> {
        self.shaders.get_mut(&id)
    }
}

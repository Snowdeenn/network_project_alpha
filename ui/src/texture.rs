use std::collections::HashMap;
use raylib::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(u16);

impl TextureId {
    pub fn value(&self) -> u16 {
        self.0
    }
}

pub struct TextureRegistry {
    pub textures: HashMap<TextureId, Texture2D>,
    next_id: u16,
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn insert(&mut self, texture: Texture2D) -> TextureId {
        debug_assert!(
            self.next_id < u16::MAX,
            "TextureRegistry has reached its maximum capacity of {} textures.",
            u16::MAX
        );
        let id = TextureId(self.next_id);
        self.textures.insert(id, texture);
        self.next_id += 1;
        id
    }

    pub fn get(&self, id: TextureId) -> Option<&Texture2D> {
        self.textures.get(&id)
    }
}
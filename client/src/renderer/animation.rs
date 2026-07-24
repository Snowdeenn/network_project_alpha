// src/renderer/animation.rs

use shared::ids::{AnimId, TextureId};
use crate::renderer::animation_manager::AnimData;

#[derive(Debug)]
pub struct AnimEntity {
    current_id: AnimId,
    current_frame: usize,
    timer: f32,
}

impl AnimEntity {
    pub fn new(id: AnimId) -> Self {
        Self {
            current_id: id,
            current_frame: 0,
            timer: 0.0,
        }
    }

    pub fn set(&mut self, id: AnimId) {
        if self.current_id != id {
            self.current_id = id;
            self.current_frame = 0;
            self.timer = 0.0;
        }
    }

    pub fn tick(&mut self, dt: f32, data: &AnimData) {
        self.timer += dt;
        if self.timer >= data.frame_time {
            self.timer = 0.0;
            let next = self.current_frame + 1;
            self.current_frame = if next < data.frames.len() {
                next
            } else if data.looping {
                0
            } else {
                self.current_frame
            };
        }
    }

    pub fn current_texture_id(&self, data: &AnimData) -> Option<TextureId> {
        data.frames.get(self.current_frame).copied()
    }
}
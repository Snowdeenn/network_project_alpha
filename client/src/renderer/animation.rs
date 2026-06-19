use crate::renderer::texture::{self, AnimData};
use raylib::texture::Texture2D;
#[derive(Debug)]
pub struct AnimEntity {
    current_id: texture::TextureId,
    current_frame: usize,
    timer: f32,
}

impl AnimEntity {
    pub fn new(id: &texture::TextureId) -> Self {
        Self {
            current_id: *id,
            current_frame: 0,
            timer: 0.0,
        }
    }

    pub fn set(&mut self, id: texture::TextureId) {
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

    pub fn current_texture<'a>(&self, data: &'a AnimData) -> &'a Texture2D {
        &data.frames[self.current_frame]
    }
}

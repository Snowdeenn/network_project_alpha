use crate::DrawCommandBuffer;

pub struct Frame {
    pub world: DrawCommandBuffer,
    pub vfx: DrawCommandBuffer,
    pub hud: DrawCommandBuffer,
    pub camera: utils::math::Mat4,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            world: DrawCommandBuffer::new(1024),
            vfx: DrawCommandBuffer::new(256),
            hud: DrawCommandBuffer::new(512),
            camera: utils::math::Mat4::identity(),
        }
    }

    pub fn clear(&mut self) {
        self.world.clear();
        self.vfx.clear();
        self.hud.clear();
    }
}
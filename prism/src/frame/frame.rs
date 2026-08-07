use crate::DrawCommandBuffer;

#[derive(Debug)]
pub struct Frame {
    pub(crate) world: DrawCommandBuffer,
    pub(crate) vfx: DrawCommandBuffer,
    pub(crate) hud: DrawCommandBuffer,
    pub camera_pos: utils::math::Vec2,
    pub cam_shake_offset: utils::math::Vec2,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            world: DrawCommandBuffer::new(1024),
            vfx: DrawCommandBuffer::new(256),
            hud: DrawCommandBuffer::new(512),
            camera_pos: utils::math::Vec2::zero(),
            cam_shake_offset: utils::math::Vec2::zero()
        }
    }

    pub fn clear(&mut self) {
        self.world.clear();
        self.vfx.clear();
        self.hud.clear();
    }

    pub fn push_world(&mut self, cmd: crate::DrawCommand) {
        self.world.push(cmd);
    }

    pub fn push_vfx(&mut self, cmd: crate::DrawCommand) {
        self.vfx.push(cmd);
    }

    pub fn push_hud(&mut self, cmd: crate::DrawCommand) {
        self.hud.push(cmd);
    }
}
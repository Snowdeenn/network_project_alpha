pub mod world;
pub mod vfx;
pub mod hud;

use crate::context::GpuContext;
use crate::draw::batch::DrawCommandBuffer;
use crate::resource::buffer::GpuBufferManager;
use crate::draw::text::TextRenderer;
use utils::math::Mat4;
pub trait Pass {
    type Input;

    /// Prépare les données CPU — tesselle, uploade vers le GPU
    fn prepare(
        &mut self,
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        input: &Self::Input,
    );

    /// Exécute la passe — crée le render pass, bind pipeline, draw
    fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        buffers: &GpuBufferManager,
    );
}

// WorldPass
pub struct WorldInput<'a> {
    pub commands: &'a DrawCommandBuffer,
    pub camera: Mat4,
}

// VfxPass
pub struct VfxInput<'a> {
    pub commands: &'a DrawCommandBuffer,
}

// HudPass  
pub struct HudInput<'a> {
    pub commands: &'a DrawCommandBuffer,
    pub text: &'a TextRenderer,
}

// PostProcessPass
pub struct PostProcessInput {
    pub source: wgpu::TextureView, // output de la WorldPass+VfxPass
}
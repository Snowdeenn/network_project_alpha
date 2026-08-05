pub mod world;
pub mod vfx;
pub mod hud;
pub mod post_process;

use crate::context::GpuContext;
use crate::draw::batch::DrawCommandBuffer;
use crate::resource::buffer::GpuBufferManager;
use utils::math::Mat4;
pub trait Pass {
    type Input<'a>
    where
        Self: 'a;

    /// Prépare les données CPU — tesselle, uploade vers le GPU
    fn prepare<'a>(
        &mut self,
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        input: &Self::Input<'a>,
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
    pub camera: Mat4,
}

// HudPass  
pub struct HudInput<'a> {
    pub commands: &'a DrawCommandBuffer,
    pub camera: Mat4,
}

// PostProcessPass
pub struct PostProcessInput<'a> {
    pub source: &'a wgpu::TextureView,
    pub target: &'a wgpu::TextureView,
}

use crate::resource::pipeline::BindGroupLayoutEntryKey;
use crate::resource::pipeline::BindingTypeKey;
pub static DEFAULT_BIND_GROUPS: &[&[BindGroupLayoutEntryKey]] = &[
    &[BindGroupLayoutEntryKey {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: BindingTypeKey::UniformBuffer,
    }],
    &[
        BindGroupLayoutEntryKey {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: BindingTypeKey::Texture2D,
        },
        BindGroupLayoutEntryKey {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: BindingTypeKey::Sampler,
        },
    ],
];
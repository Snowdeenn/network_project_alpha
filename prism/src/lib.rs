mod context;
mod draw;
mod errors;
mod frame;
mod geometry;
mod pass;
mod renderer;
mod resource;

pub use context::GpuContext;
pub use draw::{batch::DrawCommandBuffer, commands::DrawCommand, text::TextRenderer};
pub use errors::{GpuContextError, TextRendererError};
pub use frame::{frame::Frame, manager::FrameManager};
pub use geometry::{
    mesh::{RawMesh, Vertex},
    shape::{Shape, UvRect, NinePatchMargins},
    tesselator::Tesselator,
};
pub use pass::{
    HudInput, PostProcessInput, VfxInput, WorldInput, hud::HudPass, post_process::PostProcessPass,
    vfx::VfxPass, world::WorldPass,
    Pass,
};
pub use renderer::{DoubleBufferIndex, Renderer};
pub use resource::{
    buffer::{GpuBuffer, GpuBufferManager},
    pipeline::{
        BindGroupLayoutEntryKey, BindingTypeKey, BlendMode, PipelineKey, PipelineManager,
        VertexFormat,
    },
    shader::{GpuShader, ShaderManager},
    texture::{GpuTexture, TextureManager},
    material::{Material, MaterialManager},
    GpuResources,
};
pub use errors::*;

pub use pass::CAM_BIND_GROUP;
pub use pass::TEXTURE_BIND_GROUP;
pub use pass::MATERIAL_BIND_GROUP;

//! Client-owned drawing descriptions; all GPU resource references are native Velum types.
use super::TextureId;
use velum::geometry::{Shape, UvRect};

pub use prism_legacy::Color; // Shared CPU color type required by Nodus.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Alpha,
    Additive,
}

/// A typed instance batch uploads when Velum starts recording the frame.
pub trait InstanceBatch: std::fmt::Debug {
    fn submit(
        &self,
        context: Option<velum::backend::context::ContextId>,
        phase: &mut velum::backend::object::RenderPhase,
    ) -> velum::backend::RenderResult<()>;
}
impl<T: velum::backend::ObjectInstance + std::fmt::Debug> InstanceBatch for Vec<T> {
    fn submit(
        &self,
        context: Option<velum::backend::context::ContextId>,
        phase: &mut velum::backend::object::RenderPhase,
    ) -> velum::backend::RenderResult<()> {
        velum::backend::instances::submit(context, self, phase)
    }
}

#[derive(Debug)]
pub enum Primitive {
    Instances {
        batch: Box<dyn InstanceBatch>,
        camera: super::shader_bindings::UniformSlot,
        layer: u8,
    },
    Shape {
        shape: Shape,
        blend: BlendMode,
        layer: u8,
    },
    Texture {
        flash: bool,
        id: TextureId,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        uv: Option<UvRect>,
        tint: [f32; 4],
        blend: BlendMode,
        layer: u8,
    },
    Text {
        content: String,
        pos: [f32; 2],
        size: f32,
        color: [f32; 4],
        layer: u8,
    },
    Material {
        template: velum::SharedResource<velum::backend::MaterialTemplateId>,
        layout: super::shader_bindings::MeshBindings,
        resource_bindings: super::shader_bindings::ObjectBindings,
        texture_id: Option<TextureId>,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        uv: Option<UvRect>,
        tint: [f32; 4],
        uniform_data: Vec<u8>,
        blend: BlendMode,
        layer: u8,
    },
    NinePatch {
        id: TextureId,
        pos: [f32; 2],
        size: [f32; 2],
        texture_size: [f32; 2],
        margins: velum::geometry::NinePatchMargins,
        tint: [f32; 4],
        blend: BlendMode,
        layer: u8,
    },
}

impl Primitive {
    pub fn layer(&self) -> u8 {
        match self {
            Self::Instances { layer, .. }
            | Self::Shape { layer, .. }
            | Self::Texture { layer, .. }
            | Self::Text { layer, .. }
            | Self::Material { layer, .. }
            | Self::NinePatch { layer, .. } => *layer,
        }
    }
}

#[derive(Debug, Default)]
pub struct SceneFrame {
    pub world: Vec<Primitive>,
    pub vfx: Vec<Primitive>,
    pub hud: Vec<Primitive>,
    pub camera_pos: utils::math::Vec2,
    pub cam_shake_offset: utils::math::Vec2,
    pub post_process: Vec<super::post_process::PostProcessSubmission>,
    pub post_effects: Vec<super::PostProcessRequest>,
}

impl SceneFrame {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_world(&mut self, primitive: Primitive) {
        self.world.push(primitive);
    }
    pub fn push_vfx(&mut self, primitive: Primitive) {
        self.vfx.push(primitive);
    }
    pub fn push_hud(&mut self, primitive: Primitive) {
        self.hud.push(primitive);
    }
}

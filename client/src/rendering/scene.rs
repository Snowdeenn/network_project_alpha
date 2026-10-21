//! Client-owned drawing descriptions. GPU submission uses only Prism V2.
use prism::{Shape, UvRect, ids::TextureId};

pub use prism_legacy::Color; // Shared CPU color type required by Nodus.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Alpha,
    Additive,
}

#[derive(Debug)]
pub enum Primitive {
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
        template: prism::ids::MaterialTemplateHandle,
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
        margins: prism::NinePatchMargins,
        tint: [f32; 4],
        blend: BlendMode,
        layer: u8,
    },
}

impl Primitive {
    pub fn layer(&self) -> u8 {
        match self {
            Self::Shape { layer, .. }
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
    pub post_process: Vec<prism::frame::v2::PostProcessSubmission>,
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

/// Final surface overlay encoder shared by the text and Egui integrations.
pub struct OverlayContext {
    pub encoder: wgpu::CommandEncoder,
    pub surface_view: wgpu::TextureView,
    pub surface_texture: wgpu::SurfaceTexture,
    pub size: (u32, u32),
}

impl OverlayContext {
    pub fn present(self, context: &prism::GpuContext) {
        context.queue.submit([self.encoder.finish()]);
        context.present(self.surface_texture);
    }
}

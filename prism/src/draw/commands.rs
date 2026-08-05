use utils::ids::TextureId;
use crate::geometry::mesh::RawMesh;
use crate::geometry::shape::Shape;
use crate::resource::pipeline::BlendMode;

#[derive(Debug)]
pub enum DrawCommand {
    Shape {
        shape: Shape,
        blend: BlendMode,
        layer: u8,
    },
    Mesh {
        mesh: RawMesh,
        blend: BlendMode,
        layer: u8,
    },
    Texture {
        id: TextureId,
        pos: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        uv: Option<crate::geometry::shape::UvRect>,
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
}
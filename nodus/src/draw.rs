use utils::colors;
use utils::ids::{MaterialId, TextureId};
use utils::math::Vec2;

pub enum DrawCommand {
    Rect {
        pos: Vec2,
        size: Vec2,
        color: colors::Color,
        layer: u8,
    },
    Texture {
        texture_id: TextureId,
        pos: Vec2,
        size: Vec2,
        tint: colors::Color,
        layer: u8,
    },
    Material {
        material_id: MaterialId,
        texture_id: Option<TextureId>,
        pos: Vec2,
        size: Vec2,
        tint: colors::Color,
        uniform_data: Vec<u8>, // données custom passées au shader — vide si aucun uniform
        layer: u8,
    },
    NinePatch {
        texture_id: TextureId,
        pos: Vec2,
        size: Vec2,
        margins: prism::NinePatchMargins,
        tint: colors::Color,
        layer: u8,
    },
    Text {
        text: String,
        pos: Vec2,
        font_size: f32,
        color: colors::Color,
        layer: u8,
    },
}

pub struct DrawCommandBuffer {
    buffer: Vec<DrawCommand>,
}

impl DrawCommandBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.buffer.push(command);
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn sort(&mut self) {
        self.buffer.sort_unstable_by_key(|cmd| sort_key(cmd));
    }

    pub fn collect_into(&self, frame: &mut prism::Frame) {
        for cmd in self.buffer.iter() {
            match cmd {
                DrawCommand::Rect {
                    pos,
                    size,
                    color,
                    layer,
                } => {
                    frame.push_hud(prism::DrawCommand::Shape {
                        shape: prism::Shape::Quad {
                            pos: [pos.x, pos.y],
                            size: [size.x, size.y],
                            rotation: 0.0,
                            color: [
                                (color.r as f32) / 255.0,
                                (color.g as f32) / 255.0,
                                (color.b as f32) / 255.0,
                                (color.a as f32) / 255.0
                            ],
                            uv: None,
                        },
                        blend: prism::BlendMode::Alpha,
                        layer: *layer,
                    });
                }
                DrawCommand::Texture {
                    texture_id,
                    pos,
                    size,
                    tint,
                    layer,
                } => {
                    frame.push_hud(prism::DrawCommand::Texture {
                        id: *texture_id,
                        pos: [pos.x, pos.y],
                        size: [size.x, size.y],
                        rotation: 0.0,
                        uv: None,
                        tint: [
                            (tint.r as f32) / 255.0,
                            (tint.g as f32) / 255.0,
                            (tint.b as f32) / 255.0,
                            (tint.a as f32) / 255.0,
                        ],
                        blend: prism::BlendMode::Alpha,
                        layer: *layer,
                    });
                }
                DrawCommand::Text {
                    text,
                    pos,
                    font_size,
                    color,
                    layer,
                } => {
                    frame.push_hud(prism::DrawCommand::Text {
                        content: text.to_owned(),
                        pos: [pos.x, pos.y],
                        size: *font_size,
                        color: [
                            (color.r as f32) / 255.0,
                            (color.g as f32) / 255.0,
                            (color.b as f32) / 255.0,
                            (color.a as f32) / 255.0,
                        ],
                        layer: *layer,
                    });
                }
                DrawCommand::Material {
                    material_id,
                    texture_id,
                    pos,
                    size,
                    tint,
                    uniform_data,
                    layer,
                } => {
                    frame.push_hud(prism::DrawCommand::Material {
                        material_id: *material_id,
                        texture_id: *texture_id,
                        pos: [pos.x, pos.y],
                        size: [size.x, size.y],
                        rotation: 0.0,
                        uv: None,
                        tint: [
                            (tint.r as f32) / 255.0,
                            (tint.g as f32) / 255.0,
                            (tint.b as f32) / 255.0,
                            (tint.a as f32) / 255.0,
                        ],
                        // Obliger de clone parce que prism possede ces commandes
                        uniform_data: uniform_data.clone(),
                        blend: prism::BlendMode::Alpha,
                        layer: *layer,
                    });
                }
                DrawCommand::NinePatch {
                    texture_id,
                    pos,
                    size,
                    margins,
                    tint,
                    layer,
                } => {
                    frame.push_hud(prism::DrawCommand::NinePatch {
                        id: *texture_id,
                        pos: [pos.x, pos.y],
                        size: [size.x, size.y],
                        texture_size: [size.x, size.y],
                        margins: *margins,
                        tint: [
                            (tint.r as f32) / 255.0,
                            (tint.g as f32) / 255.0,
                            (tint.b as f32) / 255.0,
                            (tint.a as f32) / 255.0,
                        ],
                        blend: prism::BlendMode::Alpha,
                        layer: *layer,
                    });
                }
            }
        }
    }
}

fn sort_key(command: &DrawCommand) -> (u8, u16, u16) {
    match command {
        DrawCommand::Rect { layer, .. } => (*layer, 0, 0),
        DrawCommand::Texture {
            layer, texture_id, ..
        } => (*layer, texture_id.index as u16, 0),
        DrawCommand::Material {
            material_id,
            texture_id,
            layer,
            ..
        } => {
            if let Some(texture_id) = texture_id {
                (*layer, texture_id.index as u16, material_id.index as u16)
            } else {
                (*layer, 0, material_id.index as u16) // Texture id a 0 par defaut si texture id est none
            }
        }
        DrawCommand::NinePatch {
            texture_id, layer, ..
        } => (*layer, texture_id.index as u16, 0),
        DrawCommand::Text { layer, .. } => (*layer, 0, 0),
    }
}

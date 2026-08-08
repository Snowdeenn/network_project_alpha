use utils::colors;
use utils::ids::{ShaderId, TextureId};
use utils::math::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NinePatchMargins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl NinePatchMargins {
    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            bottom: value,
            left: value,
            right: value,
        }
    }
}

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
    Shader {
        shader_id: ShaderId,
        pos: Vec2,
        size: Vec2,
        color: colors::Color,
        layer: u8,
    },
    ShaderTexture {
        shader_id: ShaderId,
        texture_id: TextureId,
        pos: Vec2,
        size: Vec2,
        tint: colors::Color,
        layer: u8,
    },
    NinePatch {
        texture_id: TextureId,
        pos: Vec2,
        size: Vec2,
        margins: NinePatchMargins,
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
                                1.0,
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
                },
                // DrawCommand::Shader {
                //     shader_id,
                //     pos,
                //     size,
                //     color,
                //     layer,
                // } => {
                //     frame.push_hud(prism::DrawCommand::Texture {
                //         id: (),
                //         pos: [pos.x, pos.y],
                //         size: [size.x, size.y],
                //         rotation: 0.0,
                //         uv: None,
                //         tint: [
                //             (color.r as f32) / 255.0,
                //             (color.g as f32) / 255.0,
                //             (color.b as f32) / 255.0,
                //             (color.a as f32) / 255.0,
                //         ],
                //         blend: prism::BlendMode::Alpha,
                //         layer: *layer,
                //     });
                // }
                // DrawCommand::ShaderTexture {
                //     shader_id,
                //     texture_id,
                //     pos,
                //     size,
                //     tint,
                //     layer,
                // } => {
                //     frame.push_hud(prism::DrawCommand::Texture {
                //         id: *texture_id,
                //         pos: [pos.x, pos.y],
                //         size: [size.x, size.y],
                //         rotation: 0.0,
                //         uv: None,
                //         tint: [
                //             (tint.r as f32) / 255.0,
                //             (tint.g as f32) / 255.0,
                //             (tint.b as f32) / 255.0,
                //             (tint.a as f32) / 255.0,
                //         ],
                //         blend: prism::BlendMode::Alpha,
                //         layer: *layer,
                //     });
                // }
                // DrawCommand::NinePatch {
                //     texture_id,
                //     pos,
                //     size,
                //     margins,
                //     tint,
                //     layer,
                // } => {},
                _ => () // Temp pour savoir quoi faire dans les commands commenter
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
        DrawCommand::Shader {
            layer, shader_id, ..
        } => (*layer, 0, shader_id.index as u16),
        DrawCommand::ShaderTexture {
            layer,
            shader_id,
            texture_id,
            ..
        } => (*layer, texture_id.index as u16, shader_id.index as u16),
        DrawCommand::NinePatch {
            texture_id, layer, ..
        } => (*layer, texture_id.index as u16, 0),
        DrawCommand::Text { layer, .. } => (*layer, 0, 0),
    }
}

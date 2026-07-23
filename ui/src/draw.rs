use crate::provider::{ShaderProvider, TextureProvider};
use raylib::prelude::*;
use shared::ids::{ShaderId, TextureId};

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
        pos: Vector2,
        size: Vector2,
        color: Color,
        layer: u8,
    },
    Texture {
        texture_id: TextureId,
        pos: Vector2,
        size: Vector2,
        tint: Color,
        layer: u8,
    },
    Shader {
        shader_id: ShaderId,
        pos: Vector2,
        size: Vector2,
        color: Color,
        layer: u8,
    },
    ShaderTexture {
        shader_id: ShaderId,
        texture_id: TextureId,
        pos: Vector2,
        size: Vector2,
        tint: Color,
        layer: u8,
    },
    NinePatch {
        texture_id: TextureId,
        pos: Vector2,
        size: Vector2,
        margins: NinePatchMargins,
        tint: Color,
        layer: u8,
    },
    Text {
        text: String,
        pos: Vector2,
        font_size: f32,
        color: Color,
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

    pub fn flush<S: ShaderProvider, T: TextureProvider>(
        &self,
        d: &mut RaylibDrawHandle,
        tex_reg: &T,
        shader_reg: &mut S,
    ) {
        let mut i = 0;
        while i < self.buffer.len() {
            match &self.buffer[i] {
                DrawCommand::Shader { shader_id, .. }
                | DrawCommand::ShaderTexture { shader_id, .. } => {
                    let current_shader_id = *shader_id;

                    let mut j = i + 1;
                    while j < self.buffer.len() {
                        match &self.buffer[j] {
                            DrawCommand::Shader { shader_id, .. }
                            | DrawCommand::ShaderTexture { shader_id, .. }
                                if *shader_id == current_shader_id =>
                            {
                                j += 1;
                            }
                            _ => break,
                        }
                    }

                    if let Some(shader) = shader_reg.get_shader_mut(current_shader_id) {
                        let mut shader_mode = d.begin_shader_mode(shader);

                        for cmd in &self.buffer[i..j] {
                            match cmd {
                                DrawCommand::Shader {
                                    pos, size, color, ..
                                } => {
                                    shader_mode.draw_rectangle_v(*pos, *size, *color);
                                }
                                DrawCommand::ShaderTexture {
                                    texture_id,
                                    pos,
                                    size,
                                    tint,
                                    ..
                                } => {
                                    if let Some(texture) = tex_reg.get_texture(*texture_id) {
                                        let source = Rectangle {
                                            x: 0.0,
                                            y: 0.0,
                                            width: texture.width as f32,
                                            height: texture.height as f32,
                                        };
                                        let dest = Rectangle {
                                            x: pos.x,
                                            y: pos.y,
                                            width: size.x,
                                            height: size.y,
                                        };
                                        shader_mode.draw_texture_pro(
                                            texture,
                                            source,
                                            dest,
                                            Vector2::zero(),
                                            0.0,
                                            *tint,
                                        );
                                    }
                                }
                                _ => unreachable!(
                                    "Le scan précedent garantit qu'il n'y a que des Shaders ici."
                                ),
                            }
                        }
                    }
                    i = j;
                }
                DrawCommand::Rect {
                    pos, size, color, ..
                } => {
                    d.draw_rectangle_v(*pos, *size, *color);
                    i += 1;
                }
                DrawCommand::Texture {
                    texture_id,
                    pos,
                    size,
                    tint,
                    ..
                } => {
                    if let Some(texture) = tex_reg.get_texture(*texture_id) {
                        let source = Rectangle {
                            x: 0.0,
                            y: 0.0,
                            width: texture.width as f32,
                            height: texture.height as f32,
                        };
                        let dest = Rectangle {
                            x: pos.x,
                            y: pos.y,
                            width: size.x,
                            height: size.y,
                        };
                        d.draw_texture_pro(texture, source, dest, Vector2::zero(), 0.0, *tint);
                    }
                    i += 1;
                }
                DrawCommand::NinePatch {
                    texture_id,
                    pos,
                    size,
                    margins,
                    tint,
                    ..
                } => {
                    if let Some(texture) = tex_reg.get_texture(*texture_id) {
                        let source = Rectangle {
                            x: 0.0,
                            y: 0.0,
                            width: texture.width as f32,
                            height: texture.height as f32,
                        };
                        let dest = Rectangle {
                            x: pos.x,
                            y: pos.y,
                            width: size.x,
                            height: size.y,
                        };

                        let n_patch_info = NPatchInfo {
                            source,
                            left: margins.left as i32,
                            top: margins.top as i32,
                            right: margins.right as i32,
                            bottom: margins.bottom as i32,
                            layout: NPatchLayout::NPATCH_NINE_PATCH,
                        };
                        d.draw_texture_n_patch(
                            texture,
                            n_patch_info,
                            dest,
                            Vector2::zero(),
                            0.0,
                            *tint,
                        );
                    }
                    i += 1;
                }
                DrawCommand::Text {
                    text,
                    pos,
                    font_size,
                    color,
                    ..
                } => {
                    d.draw_text(text, pos.x as i32, pos.y as i32, *font_size as i32, *color);
                    i += 1;
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

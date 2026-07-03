use raylib::prelude::{Color, Vector2};

use crate::{arena::NodeId, draw::NinePatchMargins, shader::ShaderId, texture::TextureId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
    Center,
}

pub struct LayoutProps {
    pub anchor: Anchor,
    pub offset: Vector2,
    pub size: Vector2,
    pub(crate) computed_pos: Vector2,
    pub(crate) computed_size: Vector2,
}

impl LayoutProps {
    pub fn new(anchor: Anchor, offset: Vector2, size: Vector2) -> Self {
        LayoutProps {
            anchor,
            offset,
            size,
            computed_pos: Vector2::zero(),
            computed_size: Vector2::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VisualKind {
    Rect,
    Texture {
        id: TextureId,
    },
    Shader {
        id: ShaderId,
    },
    ShaderTexture {
        shader_id: ShaderId,
        texture_id: TextureId,
    },
    NinePatch {
        id: TextureId,
        margins: NinePatchMargins,
    },
    Text {
        content: String,
        font_size: f32,
    },
}

pub struct VisualProps {
    pub kind: VisualKind,
    pub color: Color,
    pub visible: bool,
    pub opacity: f32,
}

impl Default for VisualProps {
    fn default() -> Self {
        VisualProps {
            kind: VisualKind::Rect,
            color: Color::WHITE,
            visible: true,
            opacity: 1.0,
        }
    }
}

pub struct DirtyFlags {
    pub layout_dirty: bool,
    pub visual_dirty: bool,
}

impl Default for DirtyFlags {
    fn default() -> Self {
        DirtyFlags {
            layout_dirty: true,
            visual_dirty: true,
        }
    }
}

pub struct UiNode {
    pub layout: LayoutProps,
    pub visual: VisualProps,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub dirty: DirtyFlags,
}

impl UiNode {
    pub fn new(anchor: Anchor, offset: Vector2, size: Vector2) -> Self {
        Self {
            layout: LayoutProps::new(anchor, offset, size),
            visual: VisualProps::default(),
            children: Vec::new(),
            parent: None,
            dirty: DirtyFlags::default(),
        }
    }
}

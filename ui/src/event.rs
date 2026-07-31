use crate::NodeId;
use crate::node::UiVec2;
use utils::colors;
use utils::ids::{ShaderId, TextureId};

#[derive(Clone)]
pub enum UIEvent {
    SetColor { target: NodeId, color: colors::Color },
    SetOpacity { target: NodeId, opacity: f32 },
    SetVisible { target: NodeId, visible: bool },

    SetPosition { target: NodeId, offset: UiVec2 },
    SetSize { target: NodeId, size: UiVec2 },

    SetTexture { target: NodeId, id: TextureId },
    SetShader { target: NodeId, id: ShaderId },

    SetText { target: NodeId, content: String },
}

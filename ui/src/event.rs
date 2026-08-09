use crate::NodeId;
use crate::node::UiVec2;
use utils::colors;

#[derive(Clone)]
pub enum UIEvent {
    SetColor {
        target: NodeId,
        color: colors::Color,
    },
    SetOpacity {
        target: NodeId,
        opacity: f32,
    },
    SetVisible {
        target: NodeId,
        visible: bool,
    },

    SetPosition {
        target: NodeId,
        offset: UiVec2,
    },
    SetSize {
        target: NodeId,
        size: UiVec2,
    },

    SetMaterial {
        target: NodeId,
        id: utils::ids::MaterialId,
        texture_id: Option<utils::ids::TextureId>,
        uniform_data: Vec<u8>,
    },

    SetText {
        target: NodeId,
        content: String,
    },
}

use crate::arena::NodeId;
use crate::node::UiVec2;
use raylib::prelude::{Color};
use crate::texture::TextureId;
use crate::shader::ShaderId;

#[derive(Clone)]
pub enum UIEvent {
    SetColor   { target: NodeId, color: Color },
    SetOpacity { target: NodeId, opacity: f32 },
    SetVisible { target: NodeId, visible: bool },
    
    SetPosition { target: NodeId, offset: UiVec2 },
    SetSize     { target: NodeId, size: UiVec2 },
    
    SetTexture  { target: NodeId, id: TextureId },
    SetShader   { target: NodeId, id: ShaderId },

    SetText { target: NodeId, content: String }
}
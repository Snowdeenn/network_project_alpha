use crate::arena::NodeId;
use raylib::prelude::{Color, Vector2};
use crate::texture::TextureId;
use crate::shader::ShaderId;

pub enum UIEvent {
    SetColor   { target: NodeId, color: Color },
    SetOpacity { target: NodeId, opacity: f32 },
    SetVisible { target: NodeId, visible: bool },
    
    SetPosition { target: NodeId, offset: Vector2 },
    SetSize     { target: NodeId, size: Vector2 },
    
    SetTexture  { target: NodeId, id: TextureId },
    SetShader   { target: NodeId, id: ShaderId },
}
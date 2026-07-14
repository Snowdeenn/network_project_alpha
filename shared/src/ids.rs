use crate::arena::Id;

// Tags vides — zéro dépendance externe
pub struct ShaderTag;
pub struct TextureTag;

pub type ShaderId  = Id<ShaderTag>;
pub type TextureId = Id<TextureTag>;
use crate::arena::Id;
// ======================================
// Tag Client
// ======================================
// Tags vides — zéro dépendance externe
pub struct ShaderTag;
pub struct TextureTag;

pub type ShaderId  = Id<ShaderTag>;
pub type TextureId = Id<TextureTag>;

// ======================================
// Tag Server
// ======================================

pub struct EnemyTag;
pub struct CoinTag;
pub struct PlayerTag;
pub struct CooldownTag;
pub struct BufferTag;

pub type EnemyId = Id<EnemyTag>;
pub type CoinId = Id<CoinTag>;
pub type PlayerId = Id<PlayerTag>;
pub type CooldownID = Id<CooldownTag>;
pub type BufferId = Id<BufferTag>;
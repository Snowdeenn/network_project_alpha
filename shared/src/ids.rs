use crate::arena::Id;
// ======================================
// Tag Client
// ======================================
// Tags vides — zéro dépendance externe
pub struct ShaderTag;
pub struct TextureTag;
pub struct AnimTag;

pub type ShaderId = Id<ShaderTag>;
pub type TextureId = Id<TextureTag>;
pub type AnimId = Id<AnimTag>;

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

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Encode, Decode)]
pub struct InputPacket {
    pub tick_id: u64,
    pub move_dir: [f32; 2],
    pub dash: bool,
    pub spell: Option<u8>,
    pub aim_dir: [f32; 2],
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub enum ShopActionKind {
    Open,
    Buy,
    Close,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct ShopAction {
    pub kind: ShopActionKind,
    pub slot: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct ShopItem {
    pub name: String,
    pub description: String,
    pub price: u32,
    pub rarity: Rarity,
    pub effect_type: EffectType,
    pub effect_value: f32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Decode, Encode)]
pub enum EffectType {
    #[default]
    Health,
    Damage,
    Speed,
    Gold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Decode, Encode)]
pub enum BossKind {
    Big,
    Fast,
    Tank,
    Sorcerer,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub enum GameEventKind {
    WaveStart {
        wave_number: u32,
        enemy_count: u32,
        enemy_hp: u32,
        enemy_speed: f32,
    },
    WaveEnd {
        coins_earned: u32,
    },
    BossSpawn {
        entity_id: u64,
        boss_type: BossKind,
    },
    PlayerDied {
        entity_id: u64,
    },
    ShopOpened {
        inventory: Vec<Option<ShopItem>>,
    },
    ItemBought {
        item: ShopItem,
    },
    PurchaseFailed,
}

#[derive(Serialize, Deserialize, Clone, Debug, Encode, Decode)]
pub struct GameEvent {
    pub kind: GameEventKind,
}

#[derive(Debug, Serialize, Deserialize, Clone, Decode, Encode)]
pub enum EntityKind {
    Player,
    Boss(BossKind),
    Enemy,
    Projectile,
    Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct EntityState {
    pub entity_id: u64,
    pub position: [f32; 2],
    pub health: f32,
    pub max_health: f32,
    pub entity_kind: EntityKind,
    // Additional fields like velocity, status effects, etc. can be added here
}

#[derive(Debug, Deserialize, Serialize, Clone, Decode, Encode)]
pub enum WaveState {
    InProgress,
    BetweenWave { remaining_ms: u32 },
}

#[derive(Debug, Deserialize, Serialize, Clone, Decode, Encode)]
pub struct WaveInfo {
    pub wave_number: u32,
    pub enemy_remaining: u32,
    pub wave_state: WaveState,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct StateSnapshot {
    pub tick_id: u64,
    pub entities: Vec<EntityState>,
    pub wave_info: WaveInfo,
    pub player_info: Option<PlayerInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct PlayerInfo {
    pub health: f32,
    pub max_health: f32,
    pub gold: u32,
}


use std::time::Duration;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::config::PlayerClass;

#[derive(Serialize, Deserialize, Clone, Debug, Encode, Decode)]
pub struct InputPacket {
    pub move_dir: [f32; 2],
    pub aim_dir: [f32; 2],
    pub tick_id: u64,
    pub spell: Option<u8>,
    pub dash: bool,
    pub attack: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Decode, Encode, Hash)]
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
        time_between_wave: Duration,
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
        slot: usize,
    },
    PurchaseFailed {
        slot: usize,
    },
    DebugRect {
        x: f32,
        y: f32,
        half_length: f32,
        half_width: f32,
        dir: [f32; 2],
    },
    EntityHit {
        pos: [f32; 2],
    },
    DebugCollider {
        x: f32,
        y: f32,
    },
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
    Waiting,
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

#[derive(Serialize, Deserialize, Clone, Debug, Decode, Encode)]
pub struct ClassSelected {
    pub class: PlayerClass,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub enum LobbyMessage {
    // Client → Serveur (déjà dans protocol.rs)
    RequestJoinSession {
        code: String,
    },
    ClassSelected {
        class: PlayerClass,
    },
    ToggleReady,
    LeaveSession,

    // Serveur → Client (à ajouter)
    SessionJoined {
        code: String,
        slot_index: u8,
    },
    LobbyUpdate {
        slots: Vec<Option<LobbySlotInfo>>,
        phase: LobbyPhaseInfo,
    },
    GameStarting {
        countdown_secs: u8,
    },
    SessionError {
        reason: SessionErrorKind,
    },

    // In-game (à ajouter)
    SharedLivesUpdate {
        remaining: u32,
    },
    RespawnScheduled {
        player_id: u64,
        delay_secs: f32,
    },
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone)]
pub struct LobbySlotInfo {
    pub slot_index: u8,
    pub player_name: String, // pour l'instant = client_id en string, nom plus tard
    pub class: Option<PlayerClass>,
    pub ready: bool,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone, Copy)]
pub enum SessionErrorKind {
    SessionFull,
    InvalidCode,
    AlreadyInSession,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone, Copy)]
pub enum LobbyPhaseInfo {
    Waiting,
    Starting { countdown_secs: u8 },
}

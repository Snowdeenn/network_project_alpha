use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InputPacket {
    pub tick_id: u64,
    pub move_dir: [f32; 2],
    pub dash: bool,
    pub spell: Option<u8>,
    pub aim_dir: [f32; 2],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ShopActionKind {
    Buy,
    Sell,
    Close,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShopAction {
    pub kind: ShopActionKind,
    pub slot: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientMessage {
    pub input_packet: InputPacket,
    pub shop_action: Option<ShopAction>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShopItem {
    pub name: String,
    pub description: String,
    pub price: u32,
    pub rarity: Rarity,
    pub effect_type: EffectType,
    pub effect_value: f64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub enum EffectType {
    #[default]
    Health,
    Damage,
    Speed,
    Gold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum BossKind {
    Big,
    Fast,
    Tank,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameEventKind {
    WaveStart { wave_number: u32 },
    WaveEnd { coins_earned: u32 },
    BossSpawn { entity_id: u64, boss_type: BossKind },
    PlayerDied { entity_id: u64 },
    ShopOpened { inventory: Vec<ShopItem> },
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameEvent {
    pub kind: GameEventKind,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityState {
    pub entity_id: u64,
    pub position: [f32; 2],
    pub health: f32,
    pub max_health: f32,
    pub is_player: bool,
    // Additional fields like velocity, status effects, etc. can be added here
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WaveState {
    InProgress,
    BetweenWave(Duration),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WaveInfo {
    pub enemy_count: u32, 
    pub enemy_hp: u32,
    pub enemy_speed: f64,
    pub boss_spawned: bool,
    pub wave_state: WaveState,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateSnapshot {
    pub tick_id: u64,
    pub entities: Vec<EntityState>,
    pub wave_info: WaveInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerMessage {
    pub state_snapshot: StateSnapshot,
    pub game_event: Option<GameEvent>,
}
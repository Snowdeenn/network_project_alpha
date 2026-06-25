use std::collections::HashMap;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub mod protocol;
pub mod net;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Hash, Encode, Decode)]
pub enum PlayerClass {
    Warrior,
    Assassin,
    Mage,
    Tank,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SpecialKind {
    Berserker,
    Poison,
    Archimage,
    Iron,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpecialAbilityConfig {
    pub kind: SpecialKind,
    pub params: serde_json::Value,  // flexible, parsé à l'init
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColliderConfig {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovementConfig {
    pub accel: f64,
    pub max_speed: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttackConfig {
    pub range: f64,
    pub damage: u32,
    pub box_half_length: f64,
    pub box_half_width: f64,
    pub projectile_speed: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClassConfig {
    pub class: PlayerClass,
    pub max_hp: u32,
    pub collider: ColliderConfig,
    pub movement: MovementConfig,
    pub attack: AttackConfig,
    pub attack_interval_secs: f64,
    pub special: SpecialAbilityConfig,
}

#[derive(Debug)]
pub struct ClassRegistery {
    pub config: HashMap<PlayerClass, ClassConfig>,
}
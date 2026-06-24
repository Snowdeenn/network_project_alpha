use legion::Entity;
use std::str::FromStr;
use serde::Deserialize;
use std::time::Duration;
use std::collections::HashMap;

#[derive(Debug)]
pub struct WaveManager {
    pub current_wave: usize, // usize pour indexer directement dans wave_configs
    pub enemies_remaining: u32,
    pub enemies_to_spawn: u32, // ennemis pas encore spawnés cette vague
    pub spawn_timer: Duration,
    pub wave_state: WaveState,
}

#[derive(Debug)]
pub enum WaveState {
    InProgress,
    BetweenWave(Duration),
}

#[derive(Deserialize, Debug, Clone)]
pub struct WaveConfig {
    pub enemy_count: u32,
    pub enemy_hp: u32,
    pub enemy_speed: f64,
    pub spawn_interval_ms: u64,
    pub enemy_weights: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct WaveConfigs(pub Vec<WaveConfig>);

#[derive(Debug)]
pub struct EnemyPool {
    pub pool: Vec<Entity>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EnemyStatsConfig {
    pub hp_modifier: f64,
    pub speed_modifier: f64,
    pub max_speed: f64,
    pub range: f64,
    pub damage: u32,
    pub projectile_speed: Option<f64>,
    pub box_half_length: f64,
    pub box_half_width: f64,
}

pub struct EnemyConfigs(pub HashMap<String, EnemyStatsConfig>);

pub enum EnemyType {
    Melee,
    Ranged,
}

impl EnemyType {
    pub fn to_str<'a>(&self) -> &'a str {
        match self {
            EnemyType::Melee => "melee",
            EnemyType::Ranged => "ranged",
        }
    }
}

impl FromStr for EnemyType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "melee" => Ok(EnemyType::Melee),
            "ranged" => Ok(EnemyType::Ranged),
            _ => Err(format!("'{}' n'est pas un type d'ennemi valide (attendu: 'melee' ou 'ranged')", s)),
        }
    }
}

use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct WaveManager {
    pub current_wave: usize, // usize pour indexer directement dans wave_configs
    pub enemies_remaining: u32,
    pub enemies_to_spawn: u32,
    pub spawn_timer: Duration,
    pub wave_state: WaveState,
}

#[derive(Debug)]
pub enum WaveState {
    Waiting,
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
    Kamikaze,
}

impl EnemyType {
    pub fn to_str<'a>(&self) -> &'a str {
        match self {
            EnemyType::Melee => "melee",
            EnemyType::Ranged => "ranged",
            EnemyType::Kamikaze => "kamikaze",
        }
    }
}

impl FromStr for EnemyType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "melee" => Ok(EnemyType::Melee),
            "ranged" => Ok(EnemyType::Ranged),
            "kamikaze" => Ok(EnemyType::Kamikaze),
            _ => Err(format!(
                "'{}' n'est pas un type d'ennemi valide (attendu: 'melee' ou 'ranged')",
                s
            )),
        }
    }
}

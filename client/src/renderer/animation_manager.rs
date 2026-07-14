use std::collections::HashMap;

use raylib::{RaylibHandle, RaylibThread, texture::Texture2D};
use serde::Deserialize;
use shared::protocol::BossKind;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlayerState {
    Idle,
    Run,
    Dash,
    Attack,
    Die,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EnemyState {
    Idle,
    Run,
    Attack,
    Die,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum BossState {
    Idle,
    Attack,
    Run,
    Die,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum AnimId {
    Player(PlayerState),
    Enemy(EnemyState),
    Boss(BossKind, BossState),
    Coin,
    Projectile,
}

#[derive(Debug, Deserialize)]
struct AnimConfig {
    frames: Vec<String>,
    frame_time: f32,
    looping: bool,
}

#[derive(Debug)]
pub struct AnimData {
    pub frames: Vec<Texture2D>,
    pub frame_time: f32,
    pub looping: bool,
}

#[derive(Debug)]
pub struct AnimationManager {
    anims: HashMap<AnimId, AnimData>,
}

impl AnimationManager {
    pub fn load_texture(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let json = std::fs::read_to_string("assets/config/animations.json")
            .expect("[TextureManager] animations.json introuvable");
        let configs: HashMap<String, AnimConfig> = serde_json::from_str(&json)
            .expect("[Texture Manager] Impossible de deserialiser le animations.json");

        let mut anims: HashMap<AnimId, AnimData> = HashMap::new();

        for (name, cfg) in configs {
            let Ok(id) = key_to_id(name.as_str()) else {
                continue;
            };

            let frames = cfg
                .frames
                .iter()
                .filter_map(|p| rl.load_texture(thread, p).ok())
                .collect::<Vec<_>>();

            if frames.is_empty() {
                continue;
            }

            anims.insert(
                id,
                AnimData {
                    frames,
                    frame_time: cfg.frame_time,
                    looping: cfg.looping,
                },
            );
        }

        Self { anims }
    }

    pub fn get(&self, id: AnimId) -> Option<&AnimData> {
        self.anims.get(&id)
    }
}

fn id_to_key(id: &AnimId) -> &'static str {
    match id {
        AnimId::Player(PlayerState::Idle) => "player_idle",
        AnimId::Player(PlayerState::Run) => "player_run",
        AnimId::Player(PlayerState::Dash) => "player_dash",
        AnimId::Player(PlayerState::Attack) => "player_attack",
        AnimId::Player(PlayerState::Die) => "player_die",

        AnimId::Enemy(EnemyState::Idle) => "enemy_idle",
        AnimId::Enemy(EnemyState::Run) => "enemy_run",
        AnimId::Enemy(EnemyState::Attack) => "enemy_attack",
        AnimId::Enemy(EnemyState::Die) => "enemy_died",

        AnimId::Boss(BossKind::Big, BossState::Idle) => "big_boss_idle",
        AnimId::Boss(BossKind::Big, BossState::Run) => "big_boss_run",
        AnimId::Boss(BossKind::Big, BossState::Attack) => "big_boss_attack",
        AnimId::Boss(BossKind::Big, BossState::Die) => "big_boss_die",

        AnimId::Boss(BossKind::Sorcerer, BossState::Idle) => "sorcerer_boss_idle",
        AnimId::Boss(BossKind::Sorcerer, BossState::Run) => "sorcerer_boss_run",
        AnimId::Boss(BossKind::Sorcerer, BossState::Attack) => "sorcerer_boss_attack",
        AnimId::Boss(BossKind::Sorcerer, BossState::Die) => "sorcerer_boss_die",

        AnimId::Boss(BossKind::Fast, BossState::Idle) => "fast_boss_idle",
        AnimId::Boss(BossKind::Fast, BossState::Run) => "fast_boss_run",
        AnimId::Boss(BossKind::Fast, BossState::Attack) => "fast_boss_attack",
        AnimId::Boss(BossKind::Fast, BossState::Die) => "fast_boss_die",

        AnimId::Boss(BossKind::Tank, BossState::Idle) => "tank_boss_idle",
        AnimId::Boss(BossKind::Tank, BossState::Run) => "tank_boss_run",
        AnimId::Boss(BossKind::Tank, BossState::Attack) => "tank_boss_attack",
        AnimId::Boss(BossKind::Tank, BossState::Die) => "tank_boss_die",

        _ => ""
    }
}

fn key_to_id(key: &str) -> Result<AnimId, &'static str> {
    match key {
        "player_idle" => Ok(AnimId::Player(PlayerState::Idle)),
        "player_run" => Ok(AnimId::Player(PlayerState::Run)),
        "player_dash" => Ok(AnimId::Player(PlayerState::Dash)),
        "player_attack" => Ok(AnimId::Player(PlayerState::Attack)),
        "player_die" => Ok(AnimId::Player(PlayerState::Die)),

        "enemy_idle" => Ok(AnimId::Enemy(EnemyState::Idle)),
        "enemy_run" => Ok(AnimId::Enemy(EnemyState::Run)),
        "enemy_attack" => Ok(AnimId::Enemy(EnemyState::Attack)),
        "enemy_died" => Ok(AnimId::Enemy(EnemyState::Die)),

        "big_boss_idle" => Ok(AnimId::Boss(BossKind::Big, BossState::Idle)),
        "big_boss_run" => Ok(AnimId::Boss(BossKind::Big, BossState::Run)),
        "big_boss_attack" => Ok(AnimId::Boss(BossKind::Big, BossState::Attack)),
        "big_boss_die" => Ok(AnimId::Boss(BossKind::Big, BossState::Die)),

        "sorcerer_boss_idle" => Ok(AnimId::Boss(BossKind::Sorcerer, BossState::Idle)),
        "sorcerer_boss_run" => Ok(AnimId::Boss(BossKind::Sorcerer, BossState::Run)),
        "sorcerer_boss_attack" => Ok(AnimId::Boss(BossKind::Sorcerer, BossState::Attack)),
        "sorcerer_boss_die" => Ok(AnimId::Boss(BossKind::Sorcerer, BossState::Die)),

        "fast_boss_idle" => Ok(AnimId::Boss(BossKind::Fast, BossState::Idle)),
        "fast_boss_run" => Ok(AnimId::Boss(BossKind::Fast, BossState::Run)),
        "fast_boss_attack" => Ok(AnimId::Boss(BossKind::Fast, BossState::Attack)),
        "fast_boss_die" => Ok(AnimId::Boss(BossKind::Fast, BossState::Die)),

        "tank_boss_idle" => Ok(AnimId::Boss(BossKind::Tank, BossState::Idle)),
        "tank_boss_run" => Ok(AnimId::Boss(BossKind::Tank, BossState::Run)),
        "tank_boss_attack" => Ok(AnimId::Boss(BossKind::Tank, BossState::Attack)),
        "tank_boss_die" => Ok(AnimId::Boss(BossKind::Tank, BossState::Die)),

        _ => Err("Clé anim non reconnu"),
    }
}

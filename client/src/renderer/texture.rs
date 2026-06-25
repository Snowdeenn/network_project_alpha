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
pub enum TextureId {
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
pub struct TextureManager {
    anims: HashMap<TextureId, AnimData>,
}

impl TextureManager {
    pub fn load_texture(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let json = std::fs::read_to_string("assets/config/animations.json")
            .expect("[TextureManager] animations.json introuvable");
        let configs: HashMap<String, AnimConfig> = serde_json::from_str(&json)
            .expect("[Texture Manager] Impossible de deserialiser le animations.json");

        let mut anims: HashMap<TextureId, AnimData> = HashMap::new();

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

    pub fn get(&self, id: TextureId) -> Option<&AnimData> {
        self.anims.get(&id)
    }
}

fn id_to_key(id: &TextureId) -> &'static str {
    match id {
        TextureId::Player(PlayerState::Idle) => "player_idle",
        TextureId::Player(PlayerState::Run) => "player_run",
        TextureId::Player(PlayerState::Dash) => "player_dash",
        TextureId::Player(PlayerState::Attack) => "player_attack",
        TextureId::Player(PlayerState::Die) => "player_die",

        TextureId::Enemy(EnemyState::Idle) => "enemy_idle",
        TextureId::Enemy(EnemyState::Run) => "enemy_run",
        TextureId::Enemy(EnemyState::Attack) => "enemy_attack",
        TextureId::Enemy(EnemyState::Die) => "enemy_died",

        TextureId::Boss(BossKind::Big, BossState::Idle) => "big_boss_idle",
        TextureId::Boss(BossKind::Big, BossState::Run) => "big_boss_run",
        TextureId::Boss(BossKind::Big, BossState::Attack) => "big_boss_attack",
        TextureId::Boss(BossKind::Big, BossState::Die) => "big_boss_die",

        TextureId::Boss(BossKind::Sorcerer, BossState::Idle) => "sorcerer_boss_idle",
        TextureId::Boss(BossKind::Sorcerer, BossState::Run) => "sorcerer_boss_run",
        TextureId::Boss(BossKind::Sorcerer, BossState::Attack) => "sorcerer_boss_attack",
        TextureId::Boss(BossKind::Sorcerer, BossState::Die) => "sorcerer_boss_die",

        TextureId::Boss(BossKind::Fast, BossState::Idle) => "fast_boss_idle",
        TextureId::Boss(BossKind::Fast, BossState::Run) => "fast_boss_run",
        TextureId::Boss(BossKind::Fast, BossState::Attack) => "fast_boss_attack",
        TextureId::Boss(BossKind::Fast, BossState::Die) => "fast_boss_die",

        TextureId::Boss(BossKind::Tank, BossState::Idle) => "tank_boss_idle",
        TextureId::Boss(BossKind::Tank, BossState::Run) => "tank_boss_run",
        TextureId::Boss(BossKind::Tank, BossState::Attack) => "tank_boss_attack",
        TextureId::Boss(BossKind::Tank, BossState::Die) => "tank_boss_die",

        _ => ""
    }
}

fn key_to_id(key: &str) -> Result<TextureId, &'static str> {
    match key {
        "player_idle" => Ok(TextureId::Player(PlayerState::Idle)),
        "player_run" => Ok(TextureId::Player(PlayerState::Run)),
        "player_dash" => Ok(TextureId::Player(PlayerState::Dash)),
        "player_attack" => Ok(TextureId::Player(PlayerState::Attack)),
        "player_die" => Ok(TextureId::Player(PlayerState::Die)),

        "enemy_idle" => Ok(TextureId::Enemy(EnemyState::Idle)),
        "enemy_run" => Ok(TextureId::Enemy(EnemyState::Run)),
        "enemy_attack" => Ok(TextureId::Enemy(EnemyState::Attack)),
        "enemy_died" => Ok(TextureId::Enemy(EnemyState::Die)),

        "big_boss_idle" => Ok(TextureId::Boss(BossKind::Big, BossState::Idle)),
        "big_boss_run" => Ok(TextureId::Boss(BossKind::Big, BossState::Run)),
        "big_boss_attack" => Ok(TextureId::Boss(BossKind::Big, BossState::Attack)),
        "big_boss_die" => Ok(TextureId::Boss(BossKind::Big, BossState::Die)),

        "sorcerer_boss_idle" => Ok(TextureId::Boss(BossKind::Sorcerer, BossState::Idle)),
        "sorcerer_boss_run" => Ok(TextureId::Boss(BossKind::Sorcerer, BossState::Run)),
        "sorcerer_boss_attack" => Ok(TextureId::Boss(BossKind::Sorcerer, BossState::Attack)),
        "sorcerer_boss_die" => Ok(TextureId::Boss(BossKind::Sorcerer, BossState::Die)),

        "fast_boss_idle" => Ok(TextureId::Boss(BossKind::Fast, BossState::Idle)),
        "fast_boss_run" => Ok(TextureId::Boss(BossKind::Fast, BossState::Run)),
        "fast_boss_attack" => Ok(TextureId::Boss(BossKind::Fast, BossState::Attack)),
        "fast_boss_die" => Ok(TextureId::Boss(BossKind::Fast, BossState::Die)),

        "tank_boss_idle" => Ok(TextureId::Boss(BossKind::Tank, BossState::Idle)),
        "tank_boss_run" => Ok(TextureId::Boss(BossKind::Tank, BossState::Run)),
        "tank_boss_attack" => Ok(TextureId::Boss(BossKind::Tank, BossState::Attack)),
        "tank_boss_die" => Ok(TextureId::Boss(BossKind::Tank, BossState::Die)),

        _ => Err("Clé anim non reconnu"),
    }
}

// src/renderer/animation_manager.rs
use serde::Deserialize;
use std::collections::HashMap;
use utils::arena::Arena;
use utils::ids::{AnimId, AnimTag, TextureId};
use utils::protocol::BossKind;

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
pub enum AnimKey {
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


pub struct AnimData {
    pub frames: Vec<TextureId>,
    pub frame_time: f32,
    pub looping: bool,
}


pub struct AnimationManager {
    anims: Arena<AnimData, AnimTag>,
    key_map: HashMap<AnimKey, AnimId>,
    named_map: HashMap<String, AnimId>,
}

#[allow(dead_code)]
impl AnimationManager {
    pub fn new() -> Self {
        Self {
            anims: Arena::new(),
            key_map: HashMap::new(),
            named_map: HashMap::new(),
        }
    }

    pub fn load_from_config(
        &mut self,
        ctx: &prism::GpuContext,
        textures: &mut prism::TextureManager,
        config_path: &str,
    ) {
        let json = std::fs::read_to_string(config_path)
            .expect("[AnimationManager] Fichier de configuration introuvable");
        let configs: HashMap<String, AnimConfig> = serde_json::from_str(&json)
            .expect("[AnimationManager] Impossible de désérialiser le JSON d'animations");

        for (name, cfg) in configs {
            let mut frame_ids = Vec::new();

            for path in &cfg.frames {
                if let Some(tex_id) = textures.load(ctx, path) {
                    frame_ids.push(tex_id);
                }
            }

            if frame_ids.is_empty() {
                continue;
            }

            let anim_id = self.anims.insert(AnimData {
                frames: frame_ids,
                frame_time: cfg.frame_time,
                looping: cfg.looping,
            });

            if let Ok(key) = key_to_enum(name.as_str()) {
                self.key_map.insert(key, anim_id);
            }
            self.named_map.insert(name, anim_id);
        }
    }

    pub fn register(&mut self, data: AnimData) -> AnimId {
        self.anims.insert(data)
    }

    pub fn get(&self, id: AnimId) -> Option<&AnimData> {
        self.anims.get(id)
    }

    pub fn get_mut(&mut self, id: AnimId) -> Option<&mut AnimData> {
        self.anims.get_mut(id)
    }

    pub fn get_by_key(&self, key: AnimKey) -> Option<AnimId> {
        self.key_map.get(&key).copied()
    }

    pub fn get_by_name(&self, name: &str) -> Option<AnimId> {
        self.named_map.get(name).copied()
    }
}

fn key_to_enum(key: &str) -> Result<AnimKey, &'static str> {
    match key {
        "player_idle" => Ok(AnimKey::Player(PlayerState::Idle)),
        "player_run" => Ok(AnimKey::Player(PlayerState::Run)),
        "player_dash" => Ok(AnimKey::Player(PlayerState::Dash)),
        "player_attack" => Ok(AnimKey::Player(PlayerState::Attack)),
        "player_die" => Ok(AnimKey::Player(PlayerState::Die)),

        "enemy_idle" => Ok(AnimKey::Enemy(EnemyState::Idle)),
        "enemy_run" => Ok(AnimKey::Enemy(EnemyState::Run)),
        "enemy_attack" => Ok(AnimKey::Enemy(EnemyState::Attack)),
        "enemy_died" => Ok(AnimKey::Enemy(EnemyState::Die)),

        "big_boss_idle" => Ok(AnimKey::Boss(BossKind::Big, BossState::Idle)),
        "big_boss_run" => Ok(AnimKey::Boss(BossKind::Big, BossState::Run)),
        "big_boss_attack" => Ok(AnimKey::Boss(BossKind::Big, BossState::Attack)),
        "big_boss_die" => Ok(AnimKey::Boss(BossKind::Big, BossState::Die)),

        "sorcerer_boss_idle" => Ok(AnimKey::Boss(BossKind::Sorcerer, BossState::Idle)),
        "sorcerer_boss_run" => Ok(AnimKey::Boss(BossKind::Sorcerer, BossState::Run)),
        "sorcerer_boss_attack" => Ok(AnimKey::Boss(BossKind::Sorcerer, BossState::Attack)),
        "sorcerer_boss_die" => Ok(AnimKey::Boss(BossKind::Sorcerer, BossState::Die)),

        "fast_boss_idle" => Ok(AnimKey::Boss(BossKind::Fast, BossState::Idle)),
        "fast_boss_run" => Ok(AnimKey::Boss(BossKind::Fast, BossState::Run)),
        "fast_boss_attack" => Ok(AnimKey::Boss(BossKind::Fast, BossState::Attack)),
        "fast_boss_die" => Ok(AnimKey::Boss(BossKind::Fast, BossState::Die)),

        "tank_boss_idle" => Ok(AnimKey::Boss(BossKind::Tank, BossState::Idle)),
        "tank_boss_run" => Ok(AnimKey::Boss(BossKind::Tank, BossState::Run)),
        "tank_boss_attack" => Ok(AnimKey::Boss(BossKind::Tank, BossState::Attack)),
        "tank_boss_die" => Ok(AnimKey::Boss(BossKind::Tank, BossState::Die)),

        "coin" => Ok(AnimKey::Coin),
        "projectile" => Ok(AnimKey::Projectile),

        _ => Err("Clé anim non reconnue"),
    }
}

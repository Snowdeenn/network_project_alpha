use serde::Deserialize;
use std::{collections::HashMap, fmt, path::Path};

use utils::arena::Arena;
use utils::ids::{AnimId, AnimTag, TextureId};
use utils::protocol::BossKind;

#[derive(Debug)]
pub enum AnimationError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for AnimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Erreur d'E/S lors du chargement des animations : {err}"),
            Self::Json(err) => write!(f, "Erreur de désérialisation du JSON d'animations : {err}"),
        }
    }
}

impl std::error::Error for AnimationError {}

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

impl TryFrom<&str> for AnimKey {
    type Error = &'static str;

    fn try_from(key: &str) -> Result<Self, Self::Error> {
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

            _ => Err("Clé d'animation non reconnue"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AnimConfig {
    frames: Vec<String>,
    frame_time: f32,
    looping: bool,
}

#[derive(Debug, Clone)]
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

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            anims: Arena::new(),
            key_map: HashMap::new(),
            named_map: HashMap::new(),
        }
    }

    /// Charge toutes les animations définies dans un fichier de configuration JSON.
    pub fn load_from_config(
        &mut self,
        ctx: &prism::GpuContext,
        gpu_resources: &mut prism::GpuResources,
        config_path: impl AsRef<Path>,
    ) -> Result<(), AnimationError> {
        let path = config_path.as_ref();
        let _span = tracing::info_span!("AnimationManager::load_from_config", path = %path.display()).entered();

        let json = std::fs::read_to_string(path).map_err(|err| {
            tracing::error!(path = %path.display(), %err, "Impossible de lire le fichier de configuration des animations");
            AnimationError::Io(err)
        })?;

        let configs: HashMap<String, AnimConfig> = serde_json::from_str(&json).map_err(|err| {
            tracing::error!(path = %path.display(), %err, "Erreur de désérialisation du JSON d'animations");
            AnimationError::Json(err)
        })?;

        let mut loaded_count = 0;

        for (name, cfg) in configs {
            let mut frame_ids = Vec::with_capacity(cfg.frames.len());

            for frame_path in &cfg.frames {
                if let Ok(tex_id) = gpu_resources.load_texture(ctx, frame_path) {
                    frame_ids.push(tex_id);
                } else {
                    tracing::warn!(
                        anim = %name,
                        frame_path = %frame_path,
                        "Échec du chargement de la texture pour la frame"
                    );
                }
            }

            if frame_ids.is_empty() {
                tracing::warn!(anim = %name, "Animation ignorée : aucun frame n'a pu être chargé");
                continue;
            }

            let anim_id = self.anims.insert(AnimData {
                frames: frame_ids,
                frame_time: cfg.frame_time,
                looping: cfg.looping,
            });

            if let Ok(key) = AnimKey::try_from(name.as_str()) {
                self.key_map.insert(key, anim_id);
            } else {
                tracing::trace!(anim = %name, "Animation enregistrée sous son nom uniquement (sans correspondance AnimKey)");
            }

            self.named_map.insert(name, anim_id);
            loaded_count += 1;
        }

        tracing::info!(loaded = loaded_count, "Animations chargées avec succès");
        Ok(())
    }

    pub fn register(&mut self, data: AnimData) -> AnimId {
        self.anims.insert(data)
    }

    #[inline]
    pub fn get(&self, id: AnimId) -> Option<&AnimData> {
        self.anims.get(id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: AnimId) -> Option<&mut AnimData> {
        self.anims.get_mut(id)
    }

    #[inline]
    pub fn get_by_key(&self, key: AnimKey) -> Option<AnimId> {
        self.key_map.get(&key).copied()
    }

    #[inline]
    pub fn get_by_name(&self, name: &str) -> Option<AnimId> {
        self.named_map.get(name).copied()
    }
}
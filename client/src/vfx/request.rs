use math::Vec2;
use utils::Id;

use super::instance::VfxSpawnParams;

/// Identifiant de l'occurrence complète, y compris ses composants.
pub type VfxHandle = Id<super::system::RunningEffect>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpawnTicket(pub(super) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandTicket(pub(super) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfxCommandError {
    StaleHandle,
    InvalidParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualEventKey {
    pub session: u64,
    pub event: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PredictionKey {
    pub session: u64,
    pub action: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum StopMode {
    Drain,
    Immediate,
}

#[derive(Debug, Clone, Copy)]
pub enum MissingOwner {
    Drain,
    Detach,
    Immediate,
}

#[derive(Clone, Copy)]
pub struct Attachment {
    pub entity: u64,
    pub offset: Vec2,
    pub on_missing: MissingOwner,
}

#[derive(Default)]
pub struct SpawnOptions {
    /// Replace the occurrence with the same name/owner, or append to a manual trail.
    pub refresh: bool,
    pub attachment: Option<Attachment>,
    pub event_key: Option<VisualEventKey>,
    pub prediction: Option<PredictionKey>,
    /// Rejoue la séquence après extinction complète, jusqu'à un arrêt explicite.
    pub looping: bool,
}

pub enum VfxChange {
    Position(Vec2),
    Direction(Vec2),
    Scale(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfxRejection {
    UnknownEffect,
    InvalidParams,
    InstanceBudget,
    ElementBudget,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnResult {
    Spawned(VfxHandle),
    Rejected(VfxRejection),
}

pub(super) enum VfxRequest {
    Spawn {
        ticket: Option<SpawnTicket>,
        effect: String,
        params: VfxSpawnParams,
        options: SpawnOptions,
    },
    Update(CommandTicket, VfxHandle, VfxChange),
    Stop(CommandTicket, VfxHandle, StopMode),
    Reconcile {
        prediction: PredictionKey,
        confirmed: bool,
    },
    StopNamed(String),
}

impl Default for VfxSpawnParams {
    fn default() -> Self {
        Self {
            tint: [1.0; 4],
            size: None,
            duration: None,
            velocity: Vec2::zero(),
            entity: None,
            pos: Vec2::zero(),
            scale: 1.0,
            direction: Vec2::new(1.0, 0.0),
            lifetime: 1.0,
        }
    }
}

pub(super) fn valid_params(params: &VfxSpawnParams) -> bool {
    valid_position(params.pos)
        && valid_position(params.direction)
        && params.scale.is_finite()
        && params.scale >= 0.0
        && params.lifetime.is_finite()
        && (0.0..=60.0).contains(&params.lifetime)
        && params
            .tint
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        && params.size.is_none_or(|size| {
            size.iter()
                .all(|value| value.is_finite() && (0.0..=4096.0).contains(value))
        })
        && params
            .duration
            .is_none_or(|value| value.is_finite() && (0.0..=60.0).contains(&value))
        && valid_position(params.velocity)
}

pub(super) fn valid_position(pos: Vec2) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}

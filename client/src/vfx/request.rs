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
        ticket: SpawnTicket,
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
    Existing(super::compatibility::ExistingEffect),
}

impl Default for VfxSpawnParams {
    fn default() -> Self {
        Self {
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
}

pub(super) fn valid_position(pos: Vec2) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}

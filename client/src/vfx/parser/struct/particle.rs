use serde::Deserialize;

use std::collections::HashMap;

use super::{LogicalDeath, VfxEmission, VfxShape, VfxSpace};
use super::{
    VfxAnimationDisappearance, VfxBlend, VfxOrientation, VfxTransitionEvent, VfxVisualComplete,
    VfxVisualRange, VfxVisualTrigger,
};

// ============ Particle Description ==================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleInit {
    pub space: VfxSpace,
    #[serde(rename = "position")]
    pub shape: VfxShape,
    pub velocity: ParticleVelocity,
    #[serde(
        rename = "lifetime_seconds",
        deserialize_with = "super::deserialize_range"
    )]
    pub life_time_seconds: [f32; 2],
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleEvolution {
    #[serde(default)]
    pub growth: f32,
    #[serde(rename = "acceleration")]
    pub accel: [f32; 2],
    #[serde(rename = "linear_drag")]
    pub drag: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleReaction {
    on: ReactionEvent,
    actions: Vec<ReactionAction>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReactionEvent {
    Collision,
    LogicalDeath,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ReactionAction {
    SetVelocity { value: [f32; 2] },
    RequestDeath { reason: String },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleLogicalDeath {
    pub stop_simulation: bool,
    #[serde(rename = "disable_collisions")]
    pub disable_collision: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleLogicalDescriptor {
    pub emission: VfxEmission,
    #[serde(rename = "initialization")]
    pub init: ParticleInit,
    pub evolution: ParticleEvolution,
    #[serde(rename = "reactions")]
    pub reaction: Vec<ParticleReaction>,
    pub logical_death: LogicalDeath<ParticleLogicalDeath>,
}

// ====================================================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleAtlas {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ParticleAnimation {
    Flipbook {
        frames: Vec<u32>,
        fps: f32,
        #[serde(rename = "loop")]
        looping: bool,
    },
}

// ============ Particle Visual Description ==============================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ParticleRepresentationType {
    Sprite {
        texture: String,
        atlas: Option<ParticleAtlas>,
    },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: ParticleRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleAppearance {
    #[serde(default)]
    pub fade_lifetime: bool,
    #[serde(default = "super::one")]
    pub scale: f32,
    pub representation: String,
    pub blend: VfxBlend,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub orientation: VfxOrientation,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleState {
    pub animation: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleTransitionTarget {
    pub size: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: ParticleTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub animation: VfxAnimationDisappearance,
    pub orientation: VfxOrientation,
    pub alpha: VfxVisualRange,
    pub size_multiplier: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ParticleVisualDescriptor {
    pub representations: Vec<ParticleRepresentation>,
    pub appearance: ParticleAppearance,
    pub animations: HashMap<String, ParticleAnimation>,
    pub states: HashMap<String, ParticleState>,
    pub transitions: Vec<ParticleTransition>,
    pub visual_disappearance: ParticleVisualDisappearance,
}

// ====================================================================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum ParticleVelocity {
    Polar {
        #[serde(deserialize_with = "super::deserialize_range")]
        angle_degrees: [f32; 2],
        #[serde(deserialize_with = "super::deserialize_range")]
        speed: [f32; 2],
    },
    Cartesian {
        #[serde(deserialize_with = "super::deserialize_range")]
        x: [f32; 2],
        #[serde(deserialize_with = "super::deserialize_range")]
        y: [f32; 2],
    },
}

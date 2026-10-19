use serde::Deserialize;

use std::collections::HashMap;

use super::LogicalDeath;
use super::{VfxTransitionEvent, VfxVisualComplete, VfxVisualRange, VfxVisualTrigger};

// ============ PostProcess Description =====================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessInit {
    #[serde(
        rename = "lifetime_seconds",
        deserialize_with = "super::deserialize_range"
    )]
    pub life_time_seconds: [f32; 2],
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessLogicalDeath {
    pub stop_simulation: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessLogicalDescriptor {
    #[serde(rename = "initialization")]
    pub init: PostProcessInit,
    pub logical_death: LogicalDeath<PostProcessLogicalDeath>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum PostProcessParameter {
    Scalar(f32),
    Vector(Vec<f32>),
}

// ============ PostProcess Visual Description ===========================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum PostProcessRepresentationType {
    PostProcessPass { material: String },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: PostProcessRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessAppearance {
    pub representation: String,
    pub weight: f32,
    pub parameters: HashMap<String, PostProcessParameter>,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessState {}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessTransitionTarget {
    pub weight: Option<f32>,
    pub parameters: Option<HashMap<String, PostProcessParameter>>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: PostProcessTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub weight: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct PostProcessVisualDescriptor {
    pub representations: Vec<PostProcessRepresentation>,
    pub appearance: PostProcessAppearance,
    pub states: HashMap<String, PostProcessState>,
    pub transitions: Vec<PostProcessTransition>,
    pub visual_disappearance: PostProcessVisualDisappearance,
}

// ====================================================================================================

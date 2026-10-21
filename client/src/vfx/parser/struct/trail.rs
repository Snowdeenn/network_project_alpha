use serde::Deserialize;

use std::collections::HashMap;

use super::{LogicalDeath, VfxEmissionMode, VfxShape, VfxSpace};
use super::{
    VfxBlend, VfxOrientation, VfxTransitionEvent, VfxUvMode, VfxVisualComplete, VfxVisualRange,
    VfxVisualTrigger,
};

// ============ Trail Description =====================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailEmission {
    #[serde(default)]
    pub manual: bool,
    pub mode: VfxEmissionMode,
    pub rate_per_second: f32,
    pub delay_seconds: f32,
    pub duration_seconds: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailInit {
    pub space: VfxSpace,
    #[serde(rename = "position")]
    pub shape: VfxShape,
    #[serde(
        rename = "lifetime_seconds",
        deserialize_with = "super::deserialize_range"
    )]
    pub life_time_seconds: [f32; 2],
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailEvolution {
    pub point_lifetime_seconds: f32,
    pub max_points: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailLogicalDeath {
    pub stop_emission: bool,
    pub stop_simulation: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailLogicalDescription {
    pub emission: TrailEmission,
    #[serde(rename = "initialization")]
    pub init: TrailInit,
    pub evolution: TrailEvolution,
    pub logical_death: LogicalDeath<TrailLogicalDeath>,
}

// ====================================================================================================

// ============ Trail Visual Description =================================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum TrailRepresentationType {
    Ribbon { texture: String },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: TrailRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailAppearance {
    #[serde(default)]
    pub fade_points: bool,
    pub representation: String,
    pub blend: VfxBlend,
    pub width: f32,
    pub color: [f32; 4],
    pub orientation: VfxOrientation,
    pub uv_mode: VfxUvMode,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailState {}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailTransitionTarget {
    pub width: Option<f32>,
    pub color: Option<[f32; 4]>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: TrailTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub orientation: VfxOrientation,
    pub alpha: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct TrailVisualDescriptor {
    pub representations: Vec<TrailRepresentation>,
    pub appearance: TrailAppearance,
    pub states: HashMap<String, TrailState>,
    pub transitions: Vec<TrailTransition>,
    pub visual_disappearance: TrailVisualDisappearance,
}

// ====================================================================================================

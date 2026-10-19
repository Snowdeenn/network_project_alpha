use serde::Deserialize;

use std::collections::HashMap;

use super::{LogicalDeath, VfxEmission, VfxShape, VfxSpace};
use super::{VfxTransitionEvent, VfxVisualComplete, VfxVisualRange, VfxVisualTrigger};

// ============ Light Description =====================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightInit {
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
pub(crate) struct LightLogicalDeath {
    pub stop_simulation: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightLogicalDescriptor {
    pub emission: VfxEmission,
    #[serde(rename = "initialization")]
    pub init: LightInit,
    pub logical_death: LogicalDeath<LightLogicalDeath>,
}

// ============ Light Visual Description =================================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum LightRepresentationType {
    PointLight,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: LightRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightAppearance {
    pub representation: String,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub casts_shadows: bool,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightState {}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightTransitionTarget {
    pub color: Option<[f32; 3]>,
    pub intensity: Option<f32>,
    pub radius: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: LightTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub intensity: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LightVisualDescriptor {
    pub representations: Vec<LightRepresentation>,
    pub appearance: LightAppearance,
    pub states: HashMap<String, LightState>,
    pub transitions: Vec<LightTransition>,
    pub visual_disappearance: LightVisualDisappearance,
}

// ====================================================================================================

use serde::Deserialize;

use std::collections::HashMap;

use super::{LogicalDeath, VfxEmission, VfxShape, VfxSpace};
use super::{
    VfxBlend, VfxOrientation, VfxTransitionEvent, VfxVisualComplete, VfxVisualRange,
    VfxVisualTrigger,
};

// ============ Decal Description =====================================================================

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalInit {
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
pub(crate) struct DecalLogicalDeath {
    pub stop_simulation: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalLogicalDescriptor {
    pub emission: VfxEmission,
    #[serde(rename = "initialization")]
    pub init: DecalInit,
    pub logical_death: LogicalDeath<DecalLogicalDeath>,
}

// ============ Decal Visual Description =================================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DecalRepresentationType {
    Decal { texture: String },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: DecalRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalAppearance {
    pub representation: String,
    pub blend: VfxBlend,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub orientation: VfxOrientation,
    pub rotation_degrees: f32,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalState {}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalTransitionTarget {
    pub size: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
    pub rotation_degrees: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: DecalTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub orientation: VfxOrientation,
    pub alpha: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct DecalVisualDescriptor {
    pub representations: Vec<DecalRepresentation>,
    pub appearance: DecalAppearance,
    pub states: HashMap<String, DecalState>,
    pub transitions: Vec<DecalTransition>,
    pub visual_disappearance: DecalVisualDisappearance,
}

// ====================================================================================================

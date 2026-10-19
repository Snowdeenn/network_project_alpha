use serde::Deserialize;

use std::collections::HashMap;

use super::{LogicalDeath, VfxEmission, VfxShape, VfxSpace};
use super::{
    VfxBlend, VfxOrientation, VfxTransitionEvent, VfxVisualComplete, VfxVisualRange,
    VfxVisualTrigger,
};

// ============ Mesh Description ======================================================================
#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshInit {
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
pub(crate) struct MeshLogicalDeath {
    pub stop_simulation: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshLogicalDescriptor {
    pub emission: VfxEmission,
    #[serde(rename = "initialization")]
    pub init: MeshInit,
    pub logical_death: LogicalDeath<MeshLogicalDeath>,
}

// ====================================================================================================

// ============ Mesh Visual Description ==================================================================

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum MeshRepresentationType {
    Mesh { mesh: String, material: String },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshRepresentation {
    pub name: String,
    #[serde(flatten)]
    pub representation_type: MeshRepresentationType,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshAppearance {
    pub representation: String,
    pub blend: VfxBlend,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub orientation: VfxOrientation,
    pub rotation_degrees: f32,
    pub initial_state: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshState {}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshTransitionTarget {
    pub size: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
    pub rotation_degrees: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshTransition {
    pub from: String,
    pub to: String,
    pub when: VfxTransitionEvent,
    pub duration_seconds: f32,
    pub target: MeshTransitionTarget,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshVisualDisappearance {
    pub trigger: VfxVisualTrigger,
    pub duration_seconds: f32,
    pub orientation: VfxOrientation,
    pub alpha: VfxVisualRange,
    pub size_multiplier: VfxVisualRange,
    pub on_complete: VfxVisualComplete,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct MeshVisualDescriptor {
    pub representations: Vec<MeshRepresentation>,
    pub appearance: MeshAppearance,
    pub states: HashMap<String, MeshState>,
    pub transitions: Vec<MeshTransition>,
    pub visual_disappearance: MeshVisualDisappearance,
}

// ====================================================================================================

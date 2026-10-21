use serde::Deserialize;

mod decal;
mod light;
mod mesh;
pub(crate) mod particle;
mod post_process;
mod trail;

pub(crate) use decal::{DecalLogicalDescriptor, DecalVisualDescriptor};
pub(crate) use light::{LightLogicalDescriptor, LightVisualDescriptor};
pub(crate) use mesh::{MeshLogicalDescriptor, MeshRepresentationType, MeshVisualDescriptor};
pub(crate) use particle::{ParticleLogicalDescriptor, ParticleVisualDescriptor};
pub(crate) use post_process::{
    PostProcessLogicalDescriptor, PostProcessParameter, PostProcessRepresentationType,
    PostProcessVisualDescriptor,
};
pub(crate) use trail::{TrailLogicalDescription, TrailVisualDescriptor};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "family", rename_all = "kebab-case")]
pub(crate) enum VfxFamily {
    Particle {
        logical: ParticleLogicalDescriptor,
        visual: ParticleVisualDescriptor,
    },
    Mesh {
        logical: MeshLogicalDescriptor,
        visual: MeshVisualDescriptor,
    },
    Trail {
        logical: TrailLogicalDescription,
        visual: TrailVisualDescriptor,
    },
    Decal {
        logical: DecalLogicalDescriptor,
        visual: DecalVisualDescriptor,
    },
    Light {
        logical: LightLogicalDescriptor,
        visual: LightVisualDescriptor,
    },
    PostProcess {
        logical: PostProcessLogicalDescriptor,
        visual: PostProcessVisualDescriptor,
    },
    Feedback {
        feedback: FeedbackKind,
    },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum FeedbackKind {
    CameraShake { trauma: f32 },
    EntityTint { color: [f32; 4] },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxEmissionMode {
    Burst,
    Continuous,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct VfxEmission {
    pub mode: VfxEmissionMode,
    /// Nombre total pour Burst ; particules par seconde pour Continuous.
    pub count: u32,
    /// Délai initial depuis l'activation de l'instance.
    pub delay_seconds: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxSpace {
    World,
    Local,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub(crate) enum VfxShape {
    Disk {
        radius: f32,
        spawn_location: VfxSpawnLocation,
    },
    Point {
        #[serde(rename = "value")]
        coord: [f32; 2],
    },
    #[serde(rename = "attachment")]
    Attachement {
        source: String,
    }, // Enum spécifique ?
    // Box offsets are sampled independently on each axis.
    Box {
        min: [f32; 2],
        max: [f32; 2],
    },
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct LogicalDeath<D> {
    pub when: Vec<String>,
    pub on_death: D,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct VfxEffectDescriptor {
    pub version: u32,
    pub name: String,
    #[serde(flatten)]
    pub(crate) family: VfxFamily,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxBlend {
    Alpha,
    Additive,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxOrientation {
    Fixed,
    Velocity,
    Path,
    Freeze,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxUvMode {
    Stretch,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum VfxTransitionEvent {
    NormalizedAge { at_least: f32 },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxVisualTrigger {
    LogicalDeath,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxVisualValue {
    Current,
    #[serde(untagged)]
    Value(f32),
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct VfxVisualRange {
    pub from: VfxVisualValue,
    pub to: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxVisualComplete {
    ReleaseParticle,
    ReleaseMesh,
    ReleaseTrail,
    ReleaseDecal,
    ReleaseLight,
    ReleasePostProcess,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxAnimationDisappearance {
    FreezeCurrentFrame,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VfxSpawnLocation {
    Interior,
    Boundary,
}

// Conserve la notation [min, max] des structures Rust.
pub(super) fn one() -> f32 {
    1.0
}

fn deserialize_range<'de, D>(deserializer: D) -> Result<[f32; 2], D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize, PartialEq)]
    struct Range {
        min: f32,
        max: f32,
    }
    let range = Range::deserialize(deserializer)?;
    Ok([range.min, range.max])
}

#[cfg(test)]
mod tests;

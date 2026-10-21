//! Gameplay presets use the same registry, requests, instances and budgets as JSON effects.
use super::{
    QueueFull, SpawnOptions, VfxApi, VfxSpawnParams,
    registry::{Priority, VfxRegistry},
};
use crate::rendering::Color;
use math::Vec2;

pub const MOVEMENT_DUST: &str = "gameplay/movement_dust";
pub const ENTITY_HIT: &str = "gameplay/entity_hit";
pub const SLASH: &str = "gameplay/slash";
pub const PLAYER_HIT: &str = "gameplay/player_hit";
pub const BLIND: &str = "gameplay/blind";
pub const ENTITY_FLASH: &str = "gameplay/entity_flash";
pub const SWORD_TRAIL: &str = "gameplay/sword_trail";
pub const DASH_GHOST: &str = "gameplay/dash_ghost";
pub const CAMERA_SHAKE: &str = "gameplay/camera_shake";

impl VfxRegistry {
    pub fn register_gameplay_effects(&mut self) -> std::io::Result<()> {
        for (source, priority) in [
            (
                include_str!("../../../assets/vfx/gameplay/movement_dust.json"),
                Priority::Low,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/impact_particles.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/impact_shake.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/entity_hit.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/slash.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/dash_ghost.json"),
                Priority::Low,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/sword_trail.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/entity_flash.json"),
                Priority::Normal,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/player_hit.json"),
                Priority::Essential,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/blind.json"),
                Priority::Essential,
            ),
            (
                include_str!("../../../assets/vfx/gameplay/camera_shake.json"),
                Priority::Normal,
            ),
        ] {
            self.register_json(source, priority)?;
        }
        Ok(())
    }
}

impl VfxApi {
    pub fn movement_dust(&self, pos: Vec2, delta: Vec2) -> Result<(), QueueFull> {
        if delta.x.abs() <= 0.05 && delta.y.abs() <= 0.05 {
            return Ok(());
        }
        self.spawn_detached(
            MOVEMENT_DUST,
            VfxSpawnParams {
                pos,
                velocity: Vec2::new(-delta.x * 4.0, 0.0),
                ..Default::default()
            },
            SpawnOptions::default(),
        )
    }
    pub fn entity_hit(&self, pos: Vec2) -> Result<(), QueueFull> {
        self.spawn_detached(
            ENTITY_HIT,
            VfxSpawnParams {
                pos,
                ..Default::default()
            },
            SpawnOptions::default(),
        )
    }
    pub fn slash(
        &self,
        pos: Vec2,
        direction: Vec2,
        half_length: f32,
        half_width: f32,
    ) -> Result<(), QueueFull> {
        let inner = half_length * 0.5;
        self.spawn_detached(
            SLASH,
            VfxSpawnParams {
                pos,
                direction,
                size: Some([inner.min(half_width), inner.max(half_width)]),
                ..Default::default()
            },
            SpawnOptions::default(),
        )
    }
    pub fn player_hit(&self) -> Result<(), QueueFull> {
        self.screen_flash(0.2)
    }
    pub fn screen_flash(&self, duration: f32) -> Result<(), QueueFull> {
        self.spawn_detached(
            PLAYER_HIT,
            VfxSpawnParams {
                duration: Some(duration),
                ..Default::default()
            },
            SpawnOptions {
                refresh: true,
                ..Default::default()
            },
        )
    }
    pub fn blind(&self, duration: f32) -> Result<(), QueueFull> {
        self.spawn_detached(
            BLIND,
            VfxSpawnParams {
                duration: Some(duration),
                ..Default::default()
            },
            SpawnOptions {
                refresh: true,
                ..Default::default()
            },
        )
    }
    pub fn entity_flash(&self, entity: u64, duration: f32) -> Result<(), QueueFull> {
        self.spawn_detached(
            ENTITY_FLASH,
            VfxSpawnParams {
                entity: Some(entity),
                duration: Some(duration),
                ..Default::default()
            },
            SpawnOptions {
                refresh: true,
                ..Default::default()
            },
        )
    }
    pub fn trail_point(&self, entity: u64, pos: Vec2, color: Color) -> Result<(), QueueFull> {
        self.spawn_detached(
            SWORD_TRAIL,
            VfxSpawnParams {
                pos,
                entity: Some(entity),
                lifetime: 0.12,
                tint: rgba(color),
                ..Default::default()
            },
            SpawnOptions {
                refresh: true,
                ..Default::default()
            },
        )
    }
    pub fn dash_ghost(&self, pos: Vec2, duration: f32, color: Color) -> Result<(), QueueFull> {
        self.spawn_detached(
            DASH_GHOST,
            VfxSpawnParams {
                pos,
                duration: Some(duration),
                tint: rgba(color),
                ..Default::default()
            },
            SpawnOptions::default(),
        )
    }
    pub fn camera_shake(&self, trauma: f32) -> Result<(), QueueFull> {
        self.spawn_detached(
            CAMERA_SHAKE,
            VfxSpawnParams {
                scale: trauma,
                ..Default::default()
            },
            SpawnOptions::default(),
        )
    }
}

fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a].map(|value| value as f32 / 255.0)
}

#[cfg(test)]
#[path = "gameplay_tests.rs"]
mod tests;

#[derive(Default, Clone, Copy)]
pub struct PostProcessStatus {
    pub weight: f32,
    pub remaining_seconds: f32,
}

impl super::VfxSystem {
    pub fn post_process(&self, material_name: &str) -> PostProcessStatus {
        use super::{
            parser::r#struct::{PostProcessRepresentationType, VfxFamily},
            simulation::VfxSimulation,
        };
        let mut state = PostProcessStatus::default();
        for (_, components) in self.components() {
            for &id in components {
                let instance = self.instances.get(id).unwrap();
                let Some(VfxSimulation::PostProcess(effect)) = instance.simulation() else {
                    continue;
                };
                let VfxFamily::PostProcess { visual, .. } = &self
                    .registry
                    .assets
                    .get(instance.descriptor)
                    .unwrap()
                    .family
                else {
                    continue;
                };
                let representation = visual
                    .representations
                    .iter()
                    .find(|r| r.name == visual.appearance.representation)
                    .unwrap();
                let PostProcessRepresentationType::PostProcessPass { material } =
                    &representation.representation_type;
                if material == material_name {
                    state.weight = state.weight.max(effect.weight);
                    state.remaining_seconds = state
                        .remaining_seconds
                        .max((effect.lifetime - effect.age).max(0.0));
                }
            }
        }
        state
    }

    pub fn entity_tint(&self, entity: u64) -> Option<[f32; 4]> {
        use super::parser::r#struct::{FeedbackKind, VfxFamily};
        for (_, components) in self.components() {
            for &id in components {
                let instance = self.instances.get(id).unwrap();
                if instance.params.entity != Some(entity) || instance.simulation().is_none() {
                    continue;
                }
                if let VfxFamily::Feedback {
                    feedback: FeedbackKind::EntityTint { color },
                } = &self
                    .registry
                    .assets
                    .get(instance.descriptor)
                    .unwrap()
                    .family
                {
                    return Some(std::array::from_fn(|i| color[i] * instance.params.tint[i]));
                }
            }
        }
        None
    }
}

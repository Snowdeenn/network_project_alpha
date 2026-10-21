//! Adaptation temporaire des effets déjà utilisés en jeu. Les paramètres,
//! simulations et commandes de rendu existants sont conservés.

use utils::math::Vec2;

use crate::graphic_data::post_process_effect_type::HitFlashEffect;
use crate::rendering::camera::Camera;
use crate::rendering::vfx::particle::{Particle, ParticlePool};
use crate::rendering::vfx::vfx_manager::VfxManager;

use super::request::VfxRequest;
use super::{QueueFull, VfxApi, VfxSystem};

pub(super) enum ExistingEffect {
    MovementDust {
        pos: Vec2,
        delta: Vec2,
    },
    EntityHit {
        pos: Vec2,
    },
    Slash {
        pos: Vec2,
        direction: Vec2,
        half_length: f32,
        half_width: f32,
    },
    PlayerHit,
    EntityFlash {
        entity: u64,
        duration: f32,
    },
    TrailPoint {
        entity: u64,
        pos: Vec2,
        color: prism::Color,
    },
    DashGhost {
        pos: Vec2,
        duration: f32,
        color: prism::Color,
    },
    CameraShake {
        trauma: f32,
    },
}

impl VfxApi {
    pub fn movement_dust(&self, pos: Vec2, delta: Vec2) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::MovementDust {
            pos,
            delta,
        }))
    }

    pub fn entity_hit(&self, pos: Vec2) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::EntityHit { pos }))
    }

    pub fn slash(
        &self,
        pos: Vec2,
        direction: Vec2,
        half_length: f32,
        half_width: f32,
    ) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::Slash {
            pos,
            direction,
            half_length,
            half_width,
        }))
    }

    pub fn player_hit(&self) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::PlayerHit))
    }

    pub fn entity_flash(&self, entity: u64, duration: f32) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::EntityFlash {
            entity,
            duration,
        }))
    }

    pub fn trail_point(
        &self,
        entity: u64,
        pos: Vec2,
        color: prism::Color,
    ) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::TrailPoint {
            entity,
            pos,
            color,
        }))
    }

    pub fn dash_ghost(
        &self,
        pos: Vec2,
        duration: f32,
        color: prism::Color,
    ) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::DashGhost {
            pos,
            duration,
            color,
        }))
    }

    pub fn camera_shake(&self, trauma: f32) -> Result<(), QueueFull> {
        self.enqueue(VfxRequest::Existing(ExistingEffect::CameraShake { trauma }))
    }
}

impl VfxSystem {
    pub(crate) fn apply_existing(
        &mut self,
        particles: &mut ParticlePool,
        manager: &mut VfxManager,
        camera: &mut Camera,
        hit_flash: &mut HitFlashEffect,
    ) {
        for effect in self.take_existing() {
            match effect {
                ExistingEffect::MovementDust { pos, delta } => {
                    if !(delta.x.abs() > 0.05 || delta.y.abs() > 0.05) {
                        continue;
                    }
                    let lifetime = rand::random_range(0.18..0.32f32);
                    particles.spawn(Particle {
                        pos: Vec2::new(pos.x + rand::random_range(-20.0..20.0), pos.y + 20.0),
                        velocity: Vec2::new(
                            -delta.x * 4.0 + rand::random_range(-20.0..20.0),
                            rand::random_range(-50.0..-20.0),
                        ),
                        friction: 4.5,
                        lifetime,
                        lt_max: lifetime,
                        scale: 0.1,
                        growth: 3.5,
                        color: prism::Color::LIGHTGRAY,
                    });
                }
                ExistingEffect::EntityHit { pos } => {
                    for _ in 0..10 {
                        let angle = rand::random_range(0.0..std::f32::consts::TAU);
                        let speed = rand::random_range(80.0..160.0f32);
                        let lifetime = rand::random_range(0.1..0.25f32);
                        particles.spawn(Particle {
                            pos,
                            velocity: Vec2::new(angle.cos() * speed, angle.sin() * speed),
                            friction: 6.0,
                            lifetime,
                            lt_max: lifetime,
                            scale: 0.15,
                            growth: 5.5,
                            color: prism::Color::DARKGOLDENROD,
                        });
                    }
                    camera.shake.add_trauma(0.5);
                }
                ExistingEffect::Slash {
                    pos,
                    direction,
                    half_length,
                    half_width,
                } => {
                    manager.spawn_slash(
                        pos,
                        direction.y.atan2(direction.x).to_degrees(),
                        half_length * 0.5,
                        half_width,
                        60.0,
                        0.15,
                        prism::Color::WHITE,
                    );
                }
                ExistingEffect::PlayerHit => hit_flash.timer = hit_flash.total_duration,
                ExistingEffect::EntityFlash { entity, duration } => {
                    manager.spawn_flash(entity, duration)
                }
                ExistingEffect::TrailPoint { entity, pos, color } => {
                    manager.push_trail_point(entity, pos, color)
                }
                ExistingEffect::DashGhost {
                    pos,
                    duration,
                    color,
                } => manager.spawn_dash_ghost(pos, duration, color),
                ExistingEffect::CameraShake { trauma } => camera.shake.add_trauma(trauma),
            }
        }
    }
}

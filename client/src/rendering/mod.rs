pub mod camera;
pub mod types;
pub mod vfx;

use crate::graphic_data::animation::AnimEntityManager;
use crate::graphic_data::animation_manager::{AnimKey, BossState, EnemyState, PlayerState};

#[derive(Clone, Copy)]
pub struct ScreenScale {
    pub w: f32,
    pub h: f32,
}

impl ScreenScale {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w: w as f32,
            h: h as f32,
        }
    }

    pub fn x(&self, ratio: f32) -> i32 {
        (ratio * self.w) as i32
    }
    pub fn y(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }
    pub fn w(&self, ratio: f32) -> i32 {
        (ratio * self.w) as i32
    }
    pub fn h(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }
    pub fn font(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        self.w = w;
        self.h = h;
    }
}

pub(crate) fn render_world(
    frame: &mut prism::Frame,
    resources: &crate::app::resources::Resources,
    anim_entities: &mut AnimEntityManager,
    prev: Option<&utils::protocol::StateSnapshot>,
    curr: &utils::protocol::StateSnapshot,
    t: f32,
    dt: f32,
) {
    let assets = resources.read_resource::<crate::graphic_data::asset_manager::AssetManager>();
    anim_entities.tick_all(dt, assets.anims());

    for entity in &curr.entities {
        let prev_entity =
            prev.and_then(|p| p.entities.iter().find(|e| e.entity_id == entity.entity_id));

        let (x, y) = match prev_entity {
            Some(prev) => (
                utils::math::lerp(prev.position[0], entity.position[0], t),
                utils::math::lerp(prev.position[1], entity.position[1], t),
            ),
            None => (entity.position[0], entity.position[1]),
        };

        let anim_key = resolve_anim(&entity.entity_kind, prev_entity, entity);

        if let Some(anim_id) = assets.anims().get_by_key(anim_key) {
            let anim = anim_entities.get_or_create(entity.entity_id, anim_id);
            anim.set(anim_id);
            if let Some(data) = assets.anims().get(anim_id) {
                if let Some(tex_id) = anim.current_texture_id(data) {
                    frame.push_world(prism::DrawCommand::Texture {
                        id: tex_id,
                        // - 32.0 Parce que les coordonées sont ceux du centre du collider
                        // donc on soustrait la moitié de la taille du collider
                        pos: [x - 32.0, y - 32.0],
                        size: [64.0, 64.0], // à adapter selon la texture
                        rotation: 0.0,
                        uv: None,
                        tint: [1.0, 1.0, 1.0, 1.0],
                        blend: prism::BlendMode::Alpha,
                        layer: 1,
                    });
                    continue;
                }
            }
        }

        // Fallback géométrique
        draw_fallback(frame, &entity.entity_kind, x, y, entity);
    }

    anim_entities.retain(|id| curr.entities.iter().any(|e| e.entity_id == id));

    // Particules
    let particles = resources.read_resource::<crate::rendering::vfx::particle::ParticlePool>();
    particles.push_draw_commands(frame);
}

fn draw_fallback(
    frame: &mut prism::Frame,
    kind: &utils::protocol::EntityKind,
    x: f32,
    y: f32,
    entity: &utils::protocol::EntityState,
) {
    match kind {
        utils::protocol::EntityKind::Player => {
            frame.push_world(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [x - 20.0, y - 20.0],
                    size: [40.0, 40.0],
                    rotation: 0.0,
                    color: [0.529, 0.808, 0.922, 1.0], // SKYBLUE
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
        }
        utils::protocol::EntityKind::Enemy => {
            frame.push_world(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [x - 20.0, y - 20.0],
                    size: [40.0, 40.0],
                    rotation: 0.0,
                    color: [1.0, 0.0, 0.0, 1.0],
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
            // barre de vie
            let bar_w = 40.0 * (entity.health / entity.max_health);
            frame.push_world(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [x - 20.0, y - 30.0],
                    size: [40.0, 5.0],
                    rotation: 0.0,
                    color: [0.2, 0.2, 0.2, 1.0],
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
            frame.push_world(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [x - 20.0, y - 30.0],
                    size: [bar_w, 5.0],
                    rotation: 0.0,
                    color: [0.0, 1.0, 0.0, 1.0],
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
        }
        utils::protocol::EntityKind::Coin => {
            frame.push_world(prism::DrawCommand::Shape {
                shape: prism::Shape::Ring {
                    center: [x, y],
                    inner_r: 0.0,
                    outer_r: 10.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                    resolution: 10,
                    color: [
                        utils::colors::Color::YELLOW.r as f32 / 255.0,
                        utils::colors::Color::YELLOW.g as f32 / 255.0,
                        utils::colors::Color::YELLOW.b as f32 / 255.0,
                        utils::colors::Color::YELLOW.a as f32 / 255.0,
                    ],
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
        }
        _ => (),
    }
}

pub(crate) fn render_hud(frame: &mut prism::Frame, resources: &crate::app::resources::Resources) {
    if let crate::core::game_phase::GamePhase::BetweenWave { time_remaining, .. } =
        *resources.read_resource::<crate::core::game_phase::GamePhase>()
    {
        frame.push_hud(prism::DrawCommand::Text {
            content: format!(
                "Temps avant la prochaine vague {}s",
                time_remaining.as_secs()
            ),
            pos: [100.0, 50.0],
            size: 32.0,
            color: [1.0, 0.0, 0.0, 1.0],
            layer: 10,
        });
    }
}

fn resolve_anim(
    kind: &utils::protocol::EntityKind,
    prev: Option<&utils::protocol::EntityState>,
    curr: &utils::protocol::EntityState,
) -> AnimKey {
    match kind {
        utils::protocol::EntityKind::Player => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            crate::graphic_data::animation_manager::AnimKey::Player(if moving {
                PlayerState::Run
            } else {
                PlayerState::Idle
            })
        }
        utils::protocol::EntityKind::Enemy => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            AnimKey::Enemy(if moving {
                EnemyState::Run
            } else {
                EnemyState::Idle
            })
        }
        utils::protocol::EntityKind::Boss(b) => AnimKey::Boss(b.clone(), BossState::Idle),
        utils::protocol::EntityKind::Coin => AnimKey::Coin,
        utils::protocol::EntityKind::Projectile => AnimKey::Projectile,
    }
}

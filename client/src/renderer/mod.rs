pub mod animation;
pub mod debug_ui;
pub mod hud;
pub mod pipeline;
pub mod texture;
pub mod types;

use crate::config::*;
use crate::event::ClientState;
use crate::particle::ParticleSystem;
use crate::renderer::animation::AnimEntity;
use crate::renderer::texture::{BossState, EnemyState, PlayerState, TextureId, TextureManager};
use crate::renderer::types::*;
use raylib::prelude::*;
use raylib_imgui::RaylibGui;
use shared::protocol::{EntityKind, EntityState, StateSnapshot};
use std::collections::HashMap;

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
}

pub struct Renderer {
    pub thread: RaylibThread,
    pub rl: RaylibHandle,
    pub cam: Camera2D,
    pub screen_w: i32,
    pub screen_h: i32,
    pub screen_scale: ScreenScale,
    pub imgui: RaylibGui,
    texture: TextureManager,
    anim_entities: HashMap<u64, AnimEntity>,
}

impl Renderer {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let (mut rl, thread) = raylib::init()
            .size(screen_w, screen_h)
            .title("Project Alpha")
            //.fullscreen()
            .build();
        rl.set_target_fps(120);

        rl.set_exit_key(None);

        let monitor_id = raylib::window::get_current_monitor();
        let monitor_info = raylib::window::get_monitor_info(monitor_id).unwrap();
        println!(
            "Monitor {}: {}x{}",
            monitor_id, monitor_info.width, monitor_info.height
        );
        let real_w = monitor_info.width;
        let real_h = monitor_info.height;

        let zoom = real_h as f32 / REFERENCE_H;

        let cam = Camera2D {
            offset: Vector2::new(real_w as f32 / 2.0, real_h as f32 / 2.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom,
        };

        let texture = TextureManager::load_texture(&mut rl, &thread);

        let imgui = RaylibGui::new(&mut rl, &thread);
        Self {
            rl,
            thread,
            cam,
            screen_w: real_w,
            screen_h: real_h,
            screen_scale: ScreenScale::new(real_w, real_h),
            imgui,
            texture,
            anim_entities: HashMap::new(),
        }
    }

    pub fn render_frame(
        &mut self,
        frame: FrameState,
        client_state: &mut ClientState,
        particle_system: &ParticleSystem,
        ctx: &mut RenderContext,
    ) {
        let ui = self.imgui.begin(&mut self.rl);
        let mut d = self.rl.begin_drawing(&self.thread);
        d.clear_background(Color::BLACK);

        {
            let mut d2 = d.begin_mode2D(self.cam);
            match current {
                None => {
                    d2.draw_text("Connexion...", -80, -10, 20, Color::WHITE);
                }
                Some(curr) => match client_state.phase {
                    GamePhase::Dead => {
                        let text = " YOU'RE DEAD";
                        d2.draw_text(
                            text,
                            s.x(750.0 / 1920.0),
                            s.y(500.0 / 1080.0),
                            s.font(120.0 / 1920.0),
                            Color::RED,
                        );
                    }
                    GamePhase::GameOver => {
                        d2.draw_text("GAME OVER", s.x(0.32), s.y(0.42), s.font(0.12), Color::RED);
                        d2.draw_text(
                            "Toutes les vies sont épuisées",
                            s.x(0.35),
                            s.y(0.58),
                            s.font(0.03),
                            Color::LIGHTGRAY,
                        );
                    }
                    _ => render_world(
                        &mut d2,
                        particle_system,
                        &self.texture,
                        &mut self.anim_entities,
                        prev,
                        curr,
                        t,
                        dt,
                    ),
                },
            }
        }

        if let Some(snap) = current {
            hud::render(&mut d, snap, s, client_state);
        }

        {
            match client_state.phase {
                GamePhase::BetweenWave { time_remaining, .. } => {
                    let remaining = format!(
                        " Temps avant la prochaine vague {}s",
                        time_remaining.as_secs()
                    );
                    d.draw_text(
                        &remaining,
                        s.x(WAVE_TIMER_X),
                        s.y(WAVE_TIMER_Y),
                        s.font(WAVE_TIMER_FONT),
                        Color::RED,
                    );

                    if client_state.phase.can_show_shop() {
                        d.draw_text(
                            "Shop disponible — appuie sur G",
                            s.x(HUD_SHOP_NOTIF_X),
                            s.y(HUD_SHOP_NOTIF_Y),
                            s.font(HUD_SHOP_NOTIF_FONT),
                            Color::GOLD,
                        );
                    }
                }
                _ => (),
            }
        }

        Self::render_game_world(
            &mut d,
            &frame,
            client_state,
            particle_system,
            dt,
            self.cam,
            &self.screen_scale,
            &self.texture,
            &mut self.anim_entities,
        );
        Self::render_game_hud(&mut d, client_state, &self.screen_scale);
        Self::render_ui_frameworks(&mut d, ctx, ui, client_state, &self.cam);
        
        self.imgui.end();
        d.draw_fps(self.screen_w - 100, 20);
    }
}

fn render_world(
    d: &mut RaylibMode2D<RaylibDrawHandle>,
    particle_system: &ParticleSystem,
    textures: &TextureManager,
    anim_entities: &mut HashMap<u64, AnimEntity>,
    prev: Option<&StateSnapshot>,
    curr: &StateSnapshot,
    t: f32,
    dt: f32,
) {
    for entity in &curr.entities {
        let prev_entity =
            prev.and_then(|p| p.entities.iter().find(|e| e.entity_id == entity.entity_id));

        let (x, y) = match prev_entity {
            Some(prev) => (
                lerp(prev.position[0], entity.position[0], t),
                lerp(prev.position[1], entity.position[1], t),
            ),
            None => (entity.position[0], entity.position[1]),
        };

        let tex_id = resolve_anim(&entity.entity_kind, prev_entity, entity);

        let anim = anim_entities
            .entry(entity.entity_id)
            .or_insert_with(|| AnimEntity::new(&tex_id));

        anim.set(tex_id);

        if let Some(data) = textures.get(tex_id) {
            anim.tick(dt, data);
            let tex = anim.current_texture(data);
            let scale = 2.0;
            let scaled_w = tex.width as f32 * scale;
            let scaled_h = tex.height as f32 * scale;

            let source_rec = Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32);

            let dest_rec = Rectangle::new(x as f32, y as f32, scaled_w, scaled_h);

            let origin = Vector2::new(scaled_w / 2.0, scaled_h / 2.0);

            d.draw_texture_pro(&tex, source_rec, dest_rec, origin, 0.0, Color::WHITE);
        } else {
            draw_fallback(d, &entity.entity_kind, x, y, entity);
        }
    }

    anim_entities.retain(|id, _| curr.entities.iter().any(|e| e.entity_id == *id));

    particle_system.draw(d);
}

fn draw_fallback(
    d: &mut RaylibMode2D<RaylibDrawHandle>,
    kind: &EntityKind,
    x: f32,
    y: f32,
    entity: &EntityState,
) {
    match kind {
        EntityKind::Player => {
            d.draw_rectangle(x as i32 - 20, y as i32 - 20, 40, 40, Color::SKYBLUE);
        }
        EntityKind::Enemy => {
            d.draw_rectangle(x as i32 - 20, y as i32 - 20, 40, 40, Color::RED);
            let bar_w = 40.0 * (entity.health / entity.max_health);
            d.draw_rectangle(x as i32 - 20, y as i32 - 30, 40, 5, Color::DARKGRAY);
            d.draw_rectangle(x as i32 - 20, y as i32 - 30, bar_w as i32, 5, Color::GREEN);
        }
        EntityKind::Boss(_) => {
            d.draw_rectangle(x as i32 - 40, y as i32 - 40, 80, 80, Color::PURPLE);
        }
        EntityKind::Projectile => {
            d.draw_circle(x as i32, y as i32, 8.0, Color::WHITE);
        }
        EntityKind::Coin => {
            d.draw_circle(x as i32, y as i32, 10.0, Color::GOLD);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn resolve_anim(kind: &EntityKind, prev: Option<&EntityState>, curr: &EntityState) -> TextureId {
    match kind {
        EntityKind::Player => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            TextureId::Player(if moving {
                PlayerState::Run
            } else {
                PlayerState::Idle
            })
        }
        EntityKind::Enemy => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            TextureId::Enemy(if moving {
                EnemyState::Run
            } else {
                EnemyState::Idle
            })
        }
        EntityKind::Boss(b) => TextureId::Boss(b.clone(), BossState::Idle),
        EntityKind::Coin => TextureId::Coin,
        EntityKind::Projectile => TextureId::Projectile,
    }
}

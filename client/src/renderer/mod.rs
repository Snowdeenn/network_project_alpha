// src/renderer/mod.rs

pub mod animation;
pub mod animation_manager;
pub mod asset_manager;
pub mod debug_ui;
pub mod hud;
pub mod render_pipeline;
pub mod resources;
pub mod shader_manager;
pub mod texture_manager;
pub mod types;

use crate::config::*;
use crate::event::ClientState;
use crate::particle::ParticleSystem;
use crate::renderer::animation::AnimEntity;
use crate::renderer::animation_manager::{AnimKey, BossState, EnemyState, PlayerState};
use crate::renderer::asset_manager::AssetManager;
use crate::renderer::render_pipeline::RenderPipeline;
use crate::renderer::resources::Resources;
use crate::renderer::shader_manager::ShaderManager;
use crate::renderer::types::*;

use crate::event::GamePhase;
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
    pub resources: Resources,
    pub pipeline: RenderPipeline,
    pub anim_entities: HashMap<u64, AnimEntity>,
}

impl Renderer {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let (mut rl, thread) = raylib::init()
            .size(screen_w, screen_h)
            .title("Project Alpha")
            .build();
        rl.set_target_fps(120);
        rl.set_exit_key(None);

        let monitor_id = raylib::window::get_current_monitor();
        let monitor_info = raylib::window::get_monitor_info(monitor_id).unwrap();
        let real_w = monitor_info.width;
        let real_h = monitor_info.height;

        let zoom = real_h as f32 / REFERENCE_H;

        let cam = Camera2D {
            offset: Vector2::new(real_w as f32 / 2.0, real_h as f32 / 2.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom,
        };
        
        let mut resources = Resources::new();
        let mut assets = AssetManager::new();
        assets.load_animations(&mut rl, &thread, "assets/config/animations.json");

        resources.insert(assets);
        resources.insert(ShaderManager::new());

        let pipeline = RenderPipeline::new(&mut rl, &thread, real_w, real_h);
        let imgui = RaylibGui::new(&mut rl, &thread);

        Self {
            rl,
            thread,
            cam,
            screen_w: real_w,
            screen_h: real_h,
            screen_scale: ScreenScale::new(real_w, real_h),
            imgui,
            resources,
            pipeline,
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
        let dt = self.rl.get_frame_time();

        self.resources
            .write_resource::<ShaderManager>()
            .update_globals(dt, self.screen_w as f32, self.screen_h as f32);

        let ui = self.imgui.begin(&mut self.rl);
        let mut d = self.rl.begin_drawing(&self.thread);

        let assets = self.resources.read_resource::<AssetManager>();
        let mut shaders = self.resources.write_resource::<ShaderManager>();

        let cam = self.cam;
        let anim_entities = &mut self.anim_entities;
        let screen_scale = self.screen_scale;

        self.pipeline.execute(
            &mut d,
            &self.thread,
            &mut shaders,
            // --- PASSE WORLD ---
            |draw_target| {
                let mut d2 = draw_target.begin_mode2D(cam);

                match &frame.current {
                    None => {
                        d2.draw_text("Connexion...", -80, -10, 20, Color::WHITE);
                    }
                    Some(curr) => match client_state.phase {
                        GamePhase::Dead => {
                            d2.draw_text(
                                " YOU'RE DEAD",
                                screen_scale.x(750.0 / 1920.0),
                                screen_scale.y(500.0 / 1080.0),
                                screen_scale.font(120.0 / 1920.0),
                                Color::RED,
                            );
                        }
                        _ => {
                            let t = (frame.last_snap_time.elapsed().as_secs_f32()
                                / crate::screens::in_game::Ticks::TICK_DURATION.as_secs_f32())
                            .clamp(0.0, 1.0);

                            render_world(
                                &mut d2,
                                particle_system,
                                &assets,
                                anim_entities,
                                frame.prev,
                                curr,
                                t,
                                dt,
                            );
                        }
                    },
                }
            },
            // --- PASSE VFX ---
            |_draw_target| {
                // Rendu des effets VFX si nécessaire
            },
            // --- PASSE HUD ---
            |draw_handle| {
                Self::render_game_hud(draw_handle, client_state, &screen_scale);
            },
        );

        // UI & Debug ImGui par-dessus tout le reste
        Self::render_ui_frameworks(&mut d, ctx, ui, client_state, &self.cam);

        self.imgui.end();
        d.draw_fps(self.screen_w - 100, 20);
    }

    fn render_game_hud(
        d: &mut RaylibDrawHandle,
        client_state: &ClientState,
        screen_scale: &ScreenScale,
    ) {
        let s = screen_scale;
        if let GamePhase::BetweenWave { time_remaining, .. } = client_state.phase {
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
    }

    fn render_ui_frameworks(
        d: &mut RaylibDrawHandle,
        ctx: &mut RenderContext,
        ui: &mut imgui::Ui,
        client_state: &mut ClientState,
        cam: &Camera2D,
    ) {
        ctx.ui_ctx.collect(ctx.buffer);
        ctx.buffer.sort();
        ctx.buffer.flush(d, ctx.texture_manager, ctx.shader_manager);
        ctx.buffer.clear();

        debug_ui::process_debug(ui, d, cam, client_state);
    }
}

fn render_world(
    d: &mut RaylibMode2D<RaylibTextureMode<RaylibDrawHandle>>,
    particle_system: &ParticleSystem,
    assets: &AssetManager,
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

        let anim_key = resolve_anim(&entity.entity_kind, prev_entity, entity);

        if let Some(anim_id) = assets.anims().get_by_key(anim_key) {
            let anim = anim_entities
                .entry(entity.entity_id)
                .or_insert_with(|| AnimEntity::new(anim_id));

            anim.set(anim_id);

            if let Some(data) = assets.anims().get(anim_id) {
                anim.tick(dt, data);
                if let Some(tex_id) = anim.current_texture_id(data) {
                    if let Some(tex) = assets.textures().get(tex_id) {
                        let scale = 2.0;
                        let scaled_w = tex.width as f32 * scale;
                        let scaled_h = tex.height as f32 * scale;

                        let source_rec =
                            Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32);
                        let dest_rec = Rectangle::new(x, y, scaled_w, scaled_h);
                        let origin = Vector2::new(scaled_w / 2.0, scaled_h / 2.0);

                        d.draw_texture_pro(tex, source_rec, dest_rec, origin, 0.0, Color::WHITE);
                        continue;
                    }
                }
            }
        }

        draw_fallback(d, &entity.entity_kind, x, y, entity);
    }

    anim_entities.retain(|id, _| curr.entities.iter().any(|e| e.entity_id == *id));

    particle_system.draw(d);
}

fn draw_fallback(
    d: &mut RaylibMode2D<RaylibTextureMode<RaylibDrawHandle>>,
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

fn resolve_anim(kind: &EntityKind, prev: Option<&EntityState>, curr: &EntityState) -> AnimKey {
    match kind {
        EntityKind::Player => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            AnimKey::Player(if moving {
                PlayerState::Run
            } else {
                PlayerState::Idle
            })
        }
        EntityKind::Enemy => {
            let moving = prev.map(|p| p.position != curr.position).unwrap_or(false);
            AnimKey::Enemy(if moving {
                EnemyState::Run
            } else {
                EnemyState::Idle
            })
        }
        EntityKind::Boss(b) => AnimKey::Boss(b.clone(), BossState::Idle),
        EntityKind::Coin => AnimKey::Coin,
        EntityKind::Projectile => AnimKey::Projectile,
    }
}

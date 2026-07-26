use std::time::{Duration, Instant};
use raylib::prelude::*;
use ui::prelude::*;

use shared::ids::ShaderId;
use shared::protocol::{EntityKind, ShopAction, ShopActionKind, StateSnapshot};

use crate::rendering::camera;
use crate::core::config;
use crate::core::event::{handle_shop_ui_event, ClientState};
use crate::app::input::{self, ShopInputAction};
use crate::core::client::GameNetClient;
use crate::rendering::vfx::particle::{Particle, ParticlePool};
use crate::ui::hud::{HudIds, ShopHudIds};
use crate::rendering::shader_manager::ShaderManager;
use crate::rendering::types::{FrameState, RenderContext};
use crate::rendering::Renderer;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub struct Snapshots {
    pub prev_snapshot: Option<StateSnapshot>,
    pub last_snapshot: Option<StateSnapshot>,
    pub last_snap_time: Instant,
}

impl Default for Snapshots {
    fn default() -> Self {
        Self {
            prev_snapshot: None,
            last_snapshot: None,
            last_snap_time: Instant::now(),
        }
    }
}

pub struct Ticks {
    pub last_tick: Instant,
    pub tick_id: u64,
}

impl Ticks {
    pub const TICK_DURATION: Duration = Duration::from_millis(50);
}

impl Default for Ticks {
    fn default() -> Self {
        Self {
            last_tick: Instant::now(),
            tick_id: 0,
        }
    }
}

pub struct InGameIds {
    pub shop: ShopHudIds,
    pub hud: HudIds,
    pub shader: ShaderId,
}

pub struct GuiContext<'a> {
    pub ui_ctx: &'a mut UiContext,
    pub shader_manager: &'a mut ShaderManager,
    pub ids: &'a InGameIds,
}

pub struct InGameScene {
    pub snapshots: Snapshots,
    pub ticks: Ticks,
    pub particle_system: ParticlePool,
}
    
impl Default for InGameScene {
    fn default() -> Self {
        Self {
            snapshots: Snapshots::default(),
            ticks: Ticks::default(),
            particle_system: ParticlePool::new(),
        }
    }
}

impl InGameScene {
    pub fn update(
        &mut self,
        client: &mut GameNetClient,
        renderer: &mut Renderer,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
        dt: f32,
    ) {
        if renderer.rl.is_key_pressed(KeyboardKey::KEY_F2) {
            client_state.debug.cycle();
        }

        self.process_snapshots(client, gui);
        self.handle_ui(client, renderer, client_state, gui);
        self.handle_shop(client, renderer, client_state, gui);
        self.process_network_ticks(client, renderer);

        // Mises à jour logiques
        client_state.update_timers(dt);
        self.particle_system.update(dt);

        // Caméra & UI
        self.update_camera(renderer);
        gui.ui_ctx.update(dt);
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        client_state: &mut ClientState,
        ctx: &mut RenderContext,
    ) {
        let frame_state = FrameState {
            current: self.snapshots.last_snapshot.as_ref(),
            prev: self.snapshots.prev_snapshot.as_ref(),
            last_snap_time: self.snapshots.last_snap_time,
        };

        renderer.render_frame(frame_state, client_state, &self.particle_system, ctx);
    }


    /// Réception des snapshots, MAJ du HUD et génération des particules de mouvement
    fn process_snapshots(&mut self, client: &mut GameNetClient, gui: &mut GuiContext) {
        while let Some(snap) = client.recv_snapshot() {
            self.snapshots.prev_snapshot = self.snapshots.last_snapshot.take();
            self.snapshots.last_snapshot = Some(snap);
            self.snapshots.last_snap_time = Instant::now();
        }

        if let Some(snap) = &self.snapshots.last_snapshot {
            // MAJ Santé, Or & Shader du HUD
            if let Some(info) = &snap.player_info {
                let ratio = info.health / info.max_health;

                gui.ui_ctx.send_event(UIEvent::SetSize {
                    target: gui.ids.hud.hp_fill_id,
                    size: UiVec2::new(UiUnit::ParentPercent(ratio), UiUnit::ParentPercent(1.0)),
                });
                gui.ui_ctx.send_event(UIEvent::SetText {
                    target: gui.ids.hud.hp_text_id,
                    content: format!("{} / {}", info.health, info.max_health),
                });
                gui.ui_ctx.send_event(UIEvent::SetText {
                    target: gui.ids.hud.gold_label_id,
                    content: format!("Or {}", info.gold),
                });

                if let Some(shader) = gui.shader_manager.get_mut(gui.ids.shader) {
                    let loc = shader.get_shader_location("u_ratio");
                    shader.set_shader_value(loc, ratio);
                }
            }

            // MAJ Vagues
            let wave_info = &snap.wave_info;
            gui.ui_ctx.send_event(UIEvent::SetText {
                target: gui.ids.hud.wave_label_id,
                content: format!(
                    "Vague {} | Ennemis {}",
                    wave_info.wave_number, wave_info.enemy_remaining
                ),
            });

            // Particules de déplacement des joueurs
            for entity in &snap.entities {
                let prev_entity = self
                    .snapshots
                    .prev_snapshot
                    .as_ref()
                    .and_then(|p| p.entities.iter().find(|e| e.entity_id == entity.entity_id));

                let t = (self.snapshots.last_snap_time.elapsed().as_secs_f32()
                    / Ticks::TICK_DURATION.as_secs_f32())
                .clamp(0.0, 1.0);

                let (x, y) = match prev_entity {
                    Some(prev) => (
                        lerp(prev.position[0], entity.position[0], t),
                        lerp(prev.position[1], entity.position[1], t),
                    ),
                    None => (entity.position[0], entity.position[1]),
                };

                if matches!(entity.entity_kind, EntityKind::Player) {
                    if let Some(prev) = prev_entity {
                        let dx = entity.position[0] - prev.position[0];
                        let dy = entity.position[1] - prev.position[1];

                        if dx.abs() > 0.05 || dy.abs() > 0.05 {
                            let lifetime = rand::random_range(0.18..0.32f32);
                            self.particle_system.spawn(Particle {
                                pos: Vector2 {
                                    x: x + rand::random_range(-20.0..20.0),
                                    y: y + 20.0,
                                },
                                velocity: Vector2 {
                                    x: (-dx * 4.0) + rand::random_range(-20.0..20.0),
                                    y: rand::random_range(-50.0..-20.0),
                                },
                                friction: 4.5,
                                lifetime,
                                lt_max: lifetime,
                                scale: 0.1,
                                growth: 6.5,
                                color: Color::LIGHTGRAY,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Traitement des entrées utilisateur globales et dépouillement des événements réseau
    fn handle_ui(
        &mut self,
        client: &mut GameNetClient,
        renderer: &Renderer,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
    ) {
        let mouse_pos = renderer.rl.get_mouse_position();
        let pressed = renderer
            .rl
            .is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let released = renderer
            .rl
            .is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT);

        gui.ui_ctx.process_input(mouse_pos, pressed, released);

        client_state.debug.cleared = false;
        while let Some(event) = client.recv_event() {
            handle_shop_ui_event(&event, gui.ui_ctx, &gui.ids.shop);
            client_state.handle_event(event);
        }
    }

    /// Gestion des raccourcis et des clics d'achat dans la boutique
    fn handle_shop(
        &mut self,
        client: &mut GameNetClient,
        renderer: &Renderer,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
    ) {
        match input::handle_shop_input(&renderer.rl, client, client_state) {
            ShopInputAction::Close => {
                gui.ui_ctx.send_event(UIEvent::SetVisible {
                    target: gui.ids.shop.root,
                    visible: false,
                });
                client_state.close_shop();
            }
            ShopInputAction::Open | ShopInputAction::None => {}
        }

        if client_state.phase.can_show_shop()
            && renderer
                .rl
                .is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
        {
            let mouse_pos = renderer.rl.get_mouse_position();
            let card_y = renderer.screen_scale.y(config::SHOP_CARD_Y);
            let card_w = renderer.screen_scale.w(config::SHOP_CARD_W);
            let card_h = renderer.screen_scale.h(config::SHOP_CARD_H);

            let clicked = config::SHOP_SLOTS_X.iter().enumerate().find(|&(_, &x)| {
                let card_x = renderer.screen_scale.x(x);
                mouse_pos.x as i32 >= card_x
                    && mouse_pos.x as i32 <= card_x + card_w
                    && mouse_pos.y as i32 >= card_y
                    && mouse_pos.y as i32 <= card_y + card_h
            });

            if let Some((slot, _)) = clicked {
                client.send_shop_action(&ShopAction {
                    kind: ShopActionKind::Buy,
                    slot: slot as u8,
                });
            }
        }
    }

    /// Envoi régulier des inputs au serveur (20 Hz)
    fn process_network_ticks(&mut self, client: &mut GameNetClient, renderer: &Renderer) {
        if self.ticks.last_tick.elapsed() >= Ticks::TICK_DURATION {
            self.ticks.last_tick = Instant::now();

            if client.is_connected() {
                let packet = input::read_input(
                    &renderer.rl,
                    self.ticks.tick_id,
                    renderer.screen_w,
                    renderer.screen_h,
                );
                client.send_input(&packet);
            }

            client.flush();
            self.ticks.tick_id += 1;
        }
    }

    /// Mise à jour de la position de la caméra
    fn update_camera(&self, renderer: &mut Renderer) {
        if let Some(curr) = &self.snapshots.last_snapshot {
            let t = (self.snapshots.last_snap_time.elapsed().as_secs_f32()
                / Ticks::TICK_DURATION.as_secs_f32())
            .clamp(0.0, 1.0);
            camera::update(&mut renderer.cam, self.snapshots.prev_snapshot.as_ref(), curr, t);
        }
    }
}
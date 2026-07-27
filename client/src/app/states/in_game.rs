use raylib::prelude::*;
use std::time::{Duration, Instant};
use ui::{prelude::*};

use shared::ids::{ShaderId};
use shared::protocol::{
    EntityKind, GameEvent, GameEventKind, ShopAction, ShopActionKind, StateSnapshot,
};

use crate::app::input::{self, ShopInputAction};
use crate::app::resources::Resources;
use crate::core::client::GameNetClient;
use crate::core::config;
use crate::core::event::{ClientState, handle_shop_ui_event};
use crate::rendering::Renderer;
use crate::rendering::camera::{self, CameraShake};
use crate::rendering::shader_manager::ShaderManager;
use crate::rendering::types::{FrameState, RenderContext};
use crate::rendering::vfx::particle::{Particle, ParticlePool};
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::hud::{self, HudIds, ShopHudIds};

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

pub struct HudBuffers {
    pub hp: String,
    pub gold: String,
    pub wave: String,
}

impl HudBuffers {
    pub fn new() -> Self {
        Self {
            hp: String::with_capacity(16),
            gold: String::with_capacity(16),
            wave: String::with_capacity(32),
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
    pub shake: CameraShake,
    pub hud_buffers: HudBuffers,

}

impl Default for InGameScene {
    fn default() -> Self {
        Self {
            snapshots: Snapshots::default(),
            ticks: Ticks::default(),
            shake: CameraShake::default(),
            hud_buffers: HudBuffers::new(),
        }
    }
}


impl InGameScene {
    pub fn update(
        &mut self,
        resources: &mut Resources,
        client: &mut GameNetClient,
        renderer: &mut Renderer,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
        dt: f32,
    ) {
        if renderer.rl.is_key_pressed(KeyboardKey::KEY_F2) {
            client_state.debug.cycle();
        }

        self.process_snapshots(client, gui, resources);
        self.process_game_event(client, client_state, gui, resources);
        self.handle_ui(renderer, client_state, gui);
        self.handle_shop(client, renderer, client_state, gui);
        self.process_network_ticks(client, renderer);

        // Mises à jour logiques
        client_state.update_timers(dt);
        {
            resources.write_resource::<ParticlePool>().update(dt);
            resources.write_resource::<VfxManager>().update(dt);
        }


        // Caméra & UI
        self.update_camera(renderer);
        self.shake.update(dt);
        gui.ui_ctx.update(dt);
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        client_state: &mut ClientState,
        ctx: &mut RenderContext,
        resources: &mut Resources,
    ) {
        let frame_state = FrameState {
            current: self.snapshots.last_snapshot.as_ref(),
            prev: self.snapshots.prev_snapshot.as_ref(),
            last_snap_time: self.snapshots.last_snap_time,
        };

        renderer.render_frame(frame_state, client_state, ctx, resources);
    }

    /// Réception des snapshots, MAJ du HUD et génération des particules de mouvement
    fn process_snapshots(
        &mut self,
        client: &mut GameNetClient,
        gui: &mut GuiContext,
        resources: &mut Resources,
    ) {
        while let Some(snap) = client.recv_snapshot() {
            self.snapshots.prev_snapshot = self.snapshots.last_snapshot.take();
            self.snapshots.last_snapshot = Some(snap);
            self.snapshots.last_snap_time = Instant::now();
        }

        if let Some(snap) = &self.snapshots.last_snapshot {
            // MAJ hud
           hud::update(gui, snap, &mut self.hud_buffers);

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
                            resources.write_resource::<ParticlePool>().spawn(Particle {
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

    fn process_game_event(
        &mut self,
        client: &mut GameNetClient,
        state: &mut ClientState,
        gui: &mut GuiContext,
        resources: &mut Resources,
    ) {
        while let Some(event) = client.recv_event() {
            self.handle_vfx_event(&event, resources);
            handle_shop_ui_event(&event, gui.ui_ctx, &gui.ids.shop);
            state.handle_event(event);
        }
    }

    fn handle_vfx_event(&mut self, event: &GameEvent, resources: &mut Resources) {
        match event.kind {
            GameEventKind::EntityHit { pos } => {
                let mut particle_pool = resources.write_resource::<ParticlePool>();
                for _ in 0..10 {
                    let angle = rand::random_range(0.0..std::f32::consts::TAU);
                    let speed = rand::random_range(80.0..160.0f32);
                    let lifetime = rand::random_range(0.1..0.25f32);
                    particle_pool.spawn(Particle {
                        pos: Vector2::new(pos[0], pos[1]),
                        velocity: Vector2::new(angle.cos() * speed, angle.sin() * speed),
                        friction: 6.0,
                        lifetime,
                        lt_max: lifetime,
                        scale: 0.15,
                        growth: 5.5,
                        color: Color::DARKGOLDENROD,
                    })
                }
                self.shake.add_trauma(0.4);
            },
            GameEventKind::SpawnRect {
                x,
                y,
                half_length,
                half_width,
                dir,
            } => {
                let mut vfx = resources.write_resource::<VfxManager>();
                let angle_deg = dir[1].atan2(dir[0]).to_degrees();
                vfx.spawn_slash(
                    Vector2 { x, y },
                    angle_deg,
                    half_length * 0.5,
                    half_width,
                    60.0,
                    0.15,
                    Color::WHITE,
                );
            }
            _ => (),
        }
    }

    /// Traitement des entrées utilisateur globales et dépouillement des événements réseau
    fn handle_ui(
        &mut self,
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
            camera::update(
                &mut renderer.cam,
                self.snapshots.prev_snapshot.as_ref(),
                curr,
                t,
                &self.shake,
            );
        }
    }
}

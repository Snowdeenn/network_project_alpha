use std::time::{Duration, Instant};
use utils::math::Vec2;
use utils::protocol::{
    EntityKind, GameEvent, GameEventKind, ShopAction, ShopActionKind, StateSnapshot,
};

use crate::app::input::{self, Input, ShopInputAction};
use crate::app::resources::Resources;
use crate::core::client::GameNetClient;
use crate::core::config;
use crate::core::event::{ClientState, LocalId, handle_shop_ui_event};
use crate::graphic_data::animation::AnimEntityManager;
use crate::graphic_data::post_process_effect_type;
use crate::rendering::ScreenScale;
use crate::rendering::camera::{self, Camera};
use crate::rendering::vfx::particle::{Particle, ParticlePool};
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::hud::{self};

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

pub struct GuiContext<'a> {
    pub ui_ctx: &'a mut nodus::UiContext,
    pub gpu_resources: &'a mut prism::GpuResources,
    pub ids: &'a utils::ids::Register,
}

pub struct InGameScene {
    pub snapshots: Snapshots,
    pub ticks: Ticks,
    pub hud_buffers: HudBuffers,
    pub anim_entities: AnimEntityManager,
}

impl Default for InGameScene {
    fn default() -> Self {
        Self {
            snapshots: Snapshots::default(),
            ticks: Ticks::default(),
            hud_buffers: HudBuffers::new(),
            anim_entities: AnimEntityManager::new(),
        }
    }
}

impl InGameScene {
    pub fn update(
        &mut self,
        resources: &mut Resources,
        client: &mut GameNetClient,
        screen_size: winit::dpi::PhysicalSize<u32>,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
        input_state: &Input,
        scale: &ScreenScale,
        cam: &mut Camera,
        dt: f32,
    ) {
        self.process_snapshots(client, gui, resources);
        self.process_game_event(client, client_state, gui, resources, cam);
        self.handle_ui(client_state, gui, input_state);
        self.handle_shop(scale, client, client_state, gui, input_state);
        self.process_network_ticks(client, screen_size, input_state);

        // Mises à jour logiques
        client_state.update_timers(dt);
        {
            resources.write_resource::<ParticlePool>().update(dt);
            resources.write_resource::<VfxManager>().update(dt);
        }

        // Maj Post process effect
        // Hit Flash
        {
            let mut hit_flash =
                resources.write_resource::<post_process_effect_type::HitFlashEffect>();
            post_process_effect_type::update_hit_flash(&mut hit_flash, dt);
        }

        // Caméra & UI
        self.update_camera(cam);
    }

    pub fn render(
        &mut self,
        frame: &mut prism::Frame,
        client_state: &mut ClientState,
        resources: &mut Resources,
        dt: f32,
    ) {
        let t = (self.snapshots.last_snap_time.elapsed().as_secs_f32()
            / Ticks::TICK_DURATION.as_secs_f32())
        .clamp(0.0, 1.0);

        match &client_state.phase {
            crate::core::event::GamePhase::Dead => {
                frame.push_world(prism::DrawCommand::Text {
                    content: "YOU'RE DEAD".to_string(),
                    pos: [400.0, 300.0],
                    size: 64.0,
                    color: [1.0, 0.0, 0.0, 1.0],
                    layer: 0,
                });
            }
            _ => {
                if let Some(curr) = &self.snapshots.last_snapshot {
                    crate::rendering::render_world(
                        frame,
                        resources,
                        &mut self.anim_entities,
                        self.snapshots.prev_snapshot.as_ref(),
                        curr,
                        t,
                        dt,
                    );
                    let vfx = resources.read_resource::<VfxManager>();
                    vfx.push_draw_commands(frame);
                }
            }
        }
        crate::rendering::render_hud(frame, client_state);
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

                let player_entity_id = resources.read_resource::<LocalId>().entity_id;
                
                if entity.entity_id == player_entity_id {
                    resources.insert(utils::math::Vec2::new(entity.position[0], entity.position[1]));
                }

                let t = (self.snapshots.last_snap_time.elapsed().as_secs_f32()
                    / Ticks::TICK_DURATION.as_secs_f32())
                .clamp(0.0, 1.0);

                let (x, y) = match prev_entity {
                    Some(prev) => (
                        utils::math::lerp(prev.position[0], entity.position[0], t),
                        utils::math::lerp(prev.position[1], entity.position[1], t),
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
                                pos: Vec2 {
                                    x: x + rand::random_range(-20.0..20.0),
                                    y: y + 20.0,
                                },
                                velocity: Vec2 {
                                    x: (-dx * 4.0) + rand::random_range(-20.0..20.0),
                                    y: rand::random_range(-50.0..-20.0),
                                },
                                friction: 4.5,
                                lifetime,
                                lt_max: lifetime,
                                scale: 0.1,
                                growth: 3.5,
                                color: utils::colors::Color::LIGHTGRAY,
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
        cam: &mut Camera,
    ) {
        while let Some(event) = client.recv_event() {
            self.handle_vfx_event(&event, cam, resources);
            handle_shop_ui_event(&event, gui.ui_ctx, &gui.ids);

            if matches!(event.kind, GameEventKind::PlayerHit) {
                let mut hit_flash = resources.write_resource::<post_process_effect_type::HitFlashEffect>();
                hit_flash.timer = hit_flash.total_duration;
            }
            
            state.handle_event(event);
        }
    }

    fn handle_vfx_event(&mut self, event: &GameEvent, cam: &mut Camera, resources: &mut Resources) {
        match event.kind {
            GameEventKind::EntityHit { pos } => {
                let mut particle_pool = resources.write_resource::<ParticlePool>();
                for _ in 0..10 {
                    let angle = rand::random_range(0.0..std::f32::consts::TAU);
                    let speed = rand::random_range(80.0..160.0f32);
                    let lifetime = rand::random_range(0.1..0.25f32);
                    particle_pool.spawn(Particle {
                        pos: Vec2::new(pos[0], pos[1]),
                        velocity: Vec2::new(angle.cos() * speed, angle.sin() * speed),
                        friction: 6.0,
                        lifetime,
                        lt_max: lifetime,
                        scale: 0.15,
                        growth: 5.5,
                        color: utils::colors::Color::DARKGOLDENROD,
                    })
                }
                cam.shake.add_trauma(0.5);
            }
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
                    Vec2 { x, y },
                    angle_deg,
                    half_length * 0.5,
                    half_width,
                    60.0,
                    0.15,
                    utils::colors::Color::WHITE,
                );
            }
            _ => (),
        }
    }

    /// Traitement des entrées utilisateur globales et dépouillement des événements réseau
    fn handle_ui(
        &mut self,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
        input_state: &Input,
    ) {
        let mouse_pos = input_state.mouse_position();
        let pressed = input_state.is_mouse_pressed(winit::event::MouseButton::Left);
        let released = input_state.is_mousew_released(winit::event::MouseButton::Left);
        gui.ui_ctx
            .process_input(Vec2::new(mouse_pos.0, mouse_pos.1), pressed, released);

        client_state.debug.cleared = false;
    }

    /// Gestion des raccourcis et des clics d'achat dans la boutique
    fn handle_shop(
        &mut self,
        scale: &ScreenScale,
        client: &mut GameNetClient,
        client_state: &mut ClientState,
        gui: &mut GuiContext,
        input_state: &Input,
    ) {
        let root = match gui.ids.get::<nodus::NodeId>(crate::key::shop::ROOT) {
            Some(id) => id,
            None => {
                tracing::warn!("L'id {} est absent du register", crate::key::shop::ROOT);
                return;
            }
        };
        match input::handle_shop_input(input_state, client, client_state) {
            ShopInputAction::Close => {
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: root,
                    visible: false,
                });
                client_state.close_shop();
            }
            ShopInputAction::Open | ShopInputAction::None => {}
        }

        if client_state.phase.can_show_shop()
            && input_state.is_mouse_pressed(winit::event::MouseButton::Left)
        {
            let mouse_pos = input_state.mouse_position();
            let card_y = scale.y(config::SHOP_CARD_Y);
            let card_w = scale.w(config::SHOP_CARD_W);
            let card_h = scale.h(config::SHOP_CARD_H);

            let clicked = config::SHOP_SLOTS_X.iter().enumerate().find(|&(_, &x)| {
                let card_x = scale.x(x);
                mouse_pos.1 as i32 >= card_x
                    && mouse_pos.0 as i32 <= card_x + card_w
                    && mouse_pos.1 as i32 >= card_y
                    && mouse_pos.1 as i32 <= card_y + card_h
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
    fn process_network_ticks(
        &mut self,
        client: &mut GameNetClient,
        size: winit::dpi::PhysicalSize<u32>,
        input_state: &Input,
    ) {
        if self.ticks.last_tick.elapsed() >= Ticks::TICK_DURATION {
            self.ticks.last_tick = Instant::now();

            if client.is_connected() {
                let packet = input::read_input(
                    input_state,
                    self.ticks.tick_id,
                    size.width as i32,
                    size.height as i32,
                );
                client.send_input(&packet);
            }

            client.flush();
            self.ticks.tick_id += 1;
        }
    }

    /// Mise à jour de la position de la caméra
    fn update_camera(&self, cam: &mut Camera) {
        if let Some(curr) = &self.snapshots.last_snapshot {
            let t = (self.snapshots.last_snap_time.elapsed().as_secs_f32()
                / Ticks::TICK_DURATION.as_secs_f32())
            .clamp(0.0, 1.0);
            camera::update(cam, self.snapshots.prev_snapshot.as_ref(), curr, t);
        }
    }
}

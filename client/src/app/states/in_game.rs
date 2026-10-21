use std::time::{Duration, Instant};
use utils::math::Vec2;
use utils::protocol::{
    EntityKind, GameEvent, GameEventKind, ShopAction, ShopActionKind, StateSnapshot,
};

use crate::app::input::{self, Input, ShopInputAction};
use crate::app::resources::Resources;
use crate::core::client::GameNetClient;
use crate::core::config;
use crate::core::event::handle_shop_ui_event;
use crate::graphic_data::animation::AnimEntityManager;
use crate::rendering::ScreenScale;
use crate::rendering::camera::{self, Camera};
use crate::ui::hud;
use crate::vfx::{VfxApi, VfxSystem};

pub struct Snapshots {
    pub prev_snapshot: Option<StateSnapshot>,
    pub last_snapshot: Option<StateSnapshot>,
    pub last_snap_time: Instant,
}

impl Snapshots {
    fn interpolation_factor(&self) -> f32 {
        (self.last_snap_time.elapsed().as_secs_f32() / Ticks::TICK_DURATION.as_secs_f32())
            .clamp(0.0, 1.0)
    }
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
    pub gpu_resources: &'a mut crate::rendering::RenderAssets,
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
        gui: &mut GuiContext,
        input_state: &mut Input,
        scale: &ScreenScale,
        cam: &mut Camera,
        dt: f32,
    ) {
        self.process_snapshots(client, gui, resources);
        self.process_game_event(client, gui, resources);
        self.handle_ui(gui, input_state, resources);
        self.handle_shop(scale, client, gui, input_state, resources);
        self.process_network_ticks(client, screen_size, input_state);

        // Mises à jour logiques
        crate::core::update_state_timer(resources, dt);
        {
            let t = self.snapshots.interpolation_factor();
            let mut vfx = resources.write_resource::<VfxSystem>();
            vfx.update(dt, |id| {
                let current = self
                    .snapshots
                    .last_snapshot
                    .as_ref()?
                    .entities
                    .iter()
                    .find(|e| e.entity_id == id)?;
                let previous = self
                    .snapshots
                    .prev_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.entities.iter().find(|e| e.entity_id == id));
                let pos = previous.map_or(current.position, |previous| {
                    [
                        utils::math::lerp(previous.position[0], current.position[0], t),
                        utils::math::lerp(previous.position[1], current.position[1], t),
                    ]
                });
                Some(Vec2::new(pos[0], pos[1]))
            });
            cam.shake.add_trauma(vfx.take_camera_trauma());
        }

        // Caméra & UI
        cam.shake.update(dt);
        self.update_camera(cam);

        // Envoi de la demande de respawn au server
        Self::send_respawn_request(client, gui, resources);
    }

    pub fn render(
        &mut self,
        frame: &mut crate::rendering::SceneFrame,
        resources: &mut Resources,
        dt: f32,
        camera: &Camera,
        screen_size: winit::dpi::PhysicalSize<u32>,
    ) {
        let t = self.snapshots.interpolation_factor();

        let phase = resources.read_resource::<crate::core::game_phase::GamePhase>();
        match *phase {
            crate::core::game_phase::GamePhase::Wave
            | crate::core::game_phase::GamePhase::BetweenWave { .. } => {
                if let Some(curr) = &self.snapshots.last_snapshot {
                    crate::rendering::render_world(
                        frame,
                        resources,
                        &mut self.anim_entities,
                        self.snapshots.prev_snapshot.as_ref(),
                        curr,
                        t,
                        dt,
                        camera,
                        screen_size,
                    );
                }
            }
            _ => (), // On va gérer le game Over autre part
        }
        let snapshot = resources.read_resource::<VfxSystem>().extract();
        crate::vfx::render_bridge::append_to_frame(&snapshot, frame);
        crate::rendering::render_hud(frame, resources);
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
            hud::update(gui, snap, &mut self.hud_buffers, resources);

            let player_entity_id = resources.read_resource::<crate::core::LocalId>().entity_id;
            let t = self.snapshots.interpolation_factor();

            // Particules de déplacement des joueurs
            for entity in &snap.entities {
                let prev_entity = self
                    .snapshots
                    .prev_snapshot
                    .as_ref()
                    .and_then(|p| p.entities.iter().find(|e| e.entity_id == entity.entity_id));

                if entity.entity_id == player_entity_id {
                    resources.insert(utils::math::Vec2::new(
                        entity.position[0],
                        entity.position[1],
                    ));
                }

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

                        Self::spawn_movement_particle(resources, Vec2::new(x, y), dx, dy);
                    }
                }
            }
        }
    }

    fn spawn_movement_particle(resources: &mut Resources, pos: Vec2, dx: f32, dy: f32) {
        if !(dx.abs() > 0.05 || dy.abs() > 0.05) {
            return;
        }

        let _ = resources
            .read_resource::<VfxApi>()
            .movement_dust(pos, Vec2::new(dx, dy));
    }

    fn process_game_event(
        &mut self,
        client: &mut GameNetClient,
        gui: &mut GuiContext,
        resources: &mut Resources,
    ) {
        while let Some(event) = client.recv_event() {
            self.handle_vfx_event(&event, resources);
            handle_shop_ui_event(&event, gui.ui_ctx, &gui.ids);

            crate::core::event::handle_event(event, resources);
        }
    }

    fn handle_vfx_event(&mut self, event: &GameEvent, resources: &Resources) {
        let vfx = resources.read_resource::<VfxApi>();
        match event.kind {
            GameEventKind::EntityHit { pos } => {
                let _ = vfx.entity_hit(Vec2::new(pos[0], pos[1]));
            }
            GameEventKind::SpawnRect {
                x,
                y,
                half_length,
                half_width,
                dir,
            } => {
                let _ = vfx.slash(
                    Vec2::new(x, y),
                    Vec2::new(dir[0], dir[1]),
                    half_length,
                    half_width,
                );
            }
            GameEventKind::PlayerHit => {
                let _ = vfx.player_hit();
            }
            _ => (),
        }
    }

    /// Traitement des entrées utilisateur globales et dépouillement des événements réseau
    fn handle_ui(&mut self, gui: &mut GuiContext, input_state: &Input, resources: &mut Resources) {
        let mouse_pos = input_state.mouse_position();
        let pressed = input_state.is_mouse_pressed(winit::event::MouseButton::Left);
        let released = input_state.is_mouse_released(winit::event::MouseButton::Left);
        let output_event =
            gui.ui_ctx
                .process_input(Vec2::new(mouse_pos.0, mouse_pos.1), pressed, released);

        resources.insert(output_event);

        let mut debug = resources.write_resource::<crate::core::debug_state::DebugState>();
        debug.cleared = false;
    }

    /// Gestion des raccourcis et des clics d'achat dans la boutique
    fn handle_shop(
        &mut self,
        scale: &ScreenScale,
        client: &mut GameNetClient,
        gui: &mut GuiContext,
        input_state: &Input,
        resources: &mut Resources,
    ) {
        let root = match gui.ids.get::<nodus::NodeId>(crate::key::shop::ROOT) {
            Some(id) => id,
            None => {
                tracing::warn!("L'id {} est absent du register", crate::key::shop::ROOT);
                return;
            }
        };
        match input::handle_shop_input(input_state, client, resources) {
            ShopInputAction::Close => {
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: root,
                    visible: false,
                });
                let mut shop = resources.write_resource::<crate::core::shop_state::ShopUiState>();
                shop.close();
            }
            ShopInputAction::Open | ShopInputAction::None => {}
        }

        if resources
            .read_resource::<crate::core::game_phase::GamePhase>()
            .can_show_shop()
            && input_state.is_mouse_pressed(winit::event::MouseButton::Left)
        {
            let (mouse_x, mouse_y) = (
                input_state.mouse_position().0 as i32,
                input_state.mouse_position().1 as i32,
            );

            let card_y = scale.y(config::SHOP_CARD_Y);
            let card_w = scale.w(config::SHOP_CARD_W);
            let card_h = scale.h(config::SHOP_CARD_H);

            let clicked = config::SHOP_SLOTS_X.iter().enumerate().find(|&(_, &x)| {
                let card_x = scale.x(x);

                mouse_x >= card_x
                    && mouse_x <= card_x + card_w
                    && mouse_y >= card_y
                    && mouse_y <= card_y + card_h
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
        input_state: &mut Input,
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
            let t = self.snapshots.interpolation_factor();
            camera::update(cam, self.snapshots.prev_snapshot.as_ref(), curr, t);
        }
    }

    fn send_respawn_request(
        client: &mut GameNetClient,
        gui: &mut GuiContext,
        resources: &mut Resources,
    ) {
        let mut game_phase = resources.write_resource::<crate::core::game_phase::GamePhase>();
        let ui_state = resources.write_resource::<crate::core::ui_state::UiState>();
        let Some(respawn_timer) = ui_state.respawn_timer else {
            return;
        };
        if !matches!(*game_phase, crate::core::game_phase::GamePhase::Dead)
            || respawn_timer.round() != 0.0
        {
            return;
        }
        let client_id = resources.read_resource::<crate::app::ClientId>();
        let ui_output_event = resources.read_resource::<Vec<nodus::UIOutputEvent>>();
        let shared_lives_button = gui
            .ids
            .get::<nodus::NodeId>(crate::key::hud::RESPAWN_SHARED_LIVES_BUTTON)
            .unwrap();
        let gold_button = gui
            .ids
            .get::<nodus::NodeId>(crate::key::hud::RESPAWN_GOLD_BUTTON)
            .unwrap();

        for event in ui_output_event.iter() {
            let option = match event {
                nodus::UIOutputEvent::Clicked { id } if *id == shared_lives_button => {
                    utils::protocol::RespawnOption::UseSharedLife
                }
                nodus::UIOutputEvent::Clicked { id } if *id == gold_button => {
                    utils::protocol::RespawnOption::UseGold
                }
                _ => continue,
            };
            client.send_event(&GameEvent {
                kind: GameEventKind::RequestRespawn {
                    client_id: client_id.0,
                    option,
                },
            });
            *game_phase = crate::core::game_phase::GamePhase::Respawning;
        }
    }
}

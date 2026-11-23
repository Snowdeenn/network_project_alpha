pub mod input;
pub mod resources;
mod setup;
pub mod states;

use std::sync::Arc;
use utils::buffer::BufferManager;

use crate::app::input::Input;
use crate::app::resources::Resources;
use crate::app::states::in_game::InGameScene;
use crate::app::states::main_menu::MenuAction;
use crate::core;
use crate::core::client::GameNetClient;
use crate::core::event::SpellUseEvent;
use crate::graphic_data::ClientSpellSlots;
use crate::graphic_data::animation_manager::AnimationManager;
use crate::graphic_data::tile_map::TileMap;
use crate::rendering::ScreenScale;
use crate::rendering::camera::Camera;
use crate::ui::debug_ui::{DebugData, DebugRenderer};

fn build_camera_matrix(
    pos: utils::math::Vec2,
    shake: utils::math::Vec2,
    screen_w: f32,
    screen_h: f32,
) -> utils::math::Mat4 {
    let mut camera = velum::camera::Camera2D::new();
    camera.set_pos(pos.x - shake.x, pos.y - shake.y);
    camera.get_view_proj(screen_w, screen_h)
}

use crate::ui::hud;

#[derive(Clone, Copy)]
pub struct ClientId(pub u64);

pub struct App {
    window: Option<Arc<winit::window::Window>>,
    gpu_ctx: Option<crate::rendering::GpuContext>,
    asset_cache: Option<crate::rendering::AssetCache>,
    shared_data: velum::RenderData,
    nodus_bridge: crate::rendering::nodus_bridge::NodusBridge,
    renderer: Option<crate::rendering::AlphaRenderer>,
    debug_renderer: Option<DebugRenderer>,
    ui_ctx: Option<nodus::UiContext>,
    client: Option<GameNetClient>,
    map: Option<TileMap>,
    resource: Resources,
    debug_data: DebugData,
    client_id: ClientId,
    in_game_scene: InGameScene,
    screen: core::screen::AppScreen,
    is_solo: bool,
    input_state: Input,
    last_frame: std::time::Instant,
    scale: Option<ScreenScale>,
    cam: Camera,
}

impl App {
    pub fn new(_event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let mut resource = Resources::new();
        resource.insert(BufferManager::with_capacity(16));
        let mut vfx = crate::vfx::VfxSystem::default();
        vfx.registry
            .register_gameplay_effects()
            .expect("Les effets gameplay intégrés doivent être valides");
        resource.insert(vfx.api());
        resource.insert(vfx);
        resource.insert(nodus::DrawCommandBuffer::new(2048));
        resource.insert(core::ui_state::UiState::default());
        let spec_mod: Option<core::ui_state::SpectatorMode> = None;
        resource.insert(spec_mod);
        resource.insert(core::game_phase::GamePhase::default());
        resource.insert(core::debug_state::DebugState::default());
        resource.insert(core::LocalId::default());
        resource.insert(core::shop_state::ShopUiState::default());
        let client_id = ClientId(rand::random::<u64>());
        resource.insert(client_id);

        let client_spell_slot = ClientSpellSlots {
            slots: [None; utils::protocol::SPELL_SLOT_COUNT],
            cooldowns: [0.0; utils::protocol::SPELL_SLOT_COUNT],
        };
        resource.insert(client_spell_slot);
        resource.insert(SpellUseEvent(None));

        let last_frame = std::time::Instant::now();
        resource.insert(utils::ids::Register::new());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Echec lors de la création du runtime tokio");
        resource.insert(runtime);

        let asset_loader = crate::vfx::AssetLoader::new();
        resource.insert(asset_loader);
        resource.insert(core::spell_vfx::SpellVfxLoads::default());

        Self {
            window: None,
            renderer: None,
            debug_renderer: None,
            gpu_ctx: None,
            asset_cache: None,
            shared_data: velum::RenderData::new(),
            nodus_bridge: Default::default(),
            ui_ctx: None,
            client: None,
            map: None,
            resource,
            debug_data: DebugData::default(),
            client_id: client_id,
            in_game_scene: InGameScene::default(),
            screen: core::screen::AppScreen::MainMenu,
            is_solo: false,
            input_state: Input::new(),
            last_frame,
            scale: None,
            cam: Camera::default(),
        }
    }

    pub fn renderer(&self) -> &crate::rendering::AlphaRenderer {
        self.renderer.as_ref().unwrap()
    }

    pub fn renderer_mut(&mut self) -> &mut crate::rendering::AlphaRenderer {
        self.renderer.as_mut().unwrap()
    }

    fn redraw(&mut self) {
        let (Some(gpu_ctx), Some(asset_cache), Some(renderer), Some(window), Some(scale)) = (
            &self.gpu_ctx,
            &mut self.asset_cache,
            &mut self.renderer,
            &self.window,
            &self.scale,
        ) else {
            return;
        };
        if let Err(error) = renderer.activate(&self.screen) {
            tracing::error!(%error, "Activation de la scène Velum");
            return;
        }
        let mut frame = crate::rendering::SceneFrame::new();
        frame.camera_pos = self.cam.pos();
        frame.cam_shake_offset = self.cam.shake.offset();
        match &self.screen {
            core::screen::AppScreen::MainMenu => {
                states::main_menu::collect_labels(&mut frame, scale, self.client.is_some());
            }
            core::screen::AppScreen::InGame if self.client.is_some() && self.ui_ctx.is_some() => {
                self.in_game_scene.collect_objects(
                    &mut frame,
                    &self.resource,
                    &self.cam,
                    gpu_ctx.size,
                );
                if let Some(map) = &self.map {
                    map.collect_visible_tiles(
                        &self.resource,
                        &mut frame,
                        gpu_ctx.size.width as f32,
                        gpu_ctx.size.height as f32,
                    );
                }
            }
            _ => {}
        }
        if let Some(ui_ctx) = &mut self.ui_ctx {
            let mut buf = self.resource.write_resource::<nodus::DrawCommandBuffer>();
            hud::prepare_hud(
                &mut frame,
                ui_ctx,
                &mut buf,
                &self.nodus_bridge,
                &self.shared_data,
            );
        }
        match renderer.render(gpu_ctx, asset_cache, &mut self.shared_data, frame) {
            Ok(Some(mut frame_ctx)) => {
                if let core::screen::AppScreen::InGame = &self.screen {
                    let mode = self
                        .resource
                        .read_resource::<crate::core::debug_state::DebugState>()
                        .mode;
                    if mode != crate::core::debug_state::DebugMode::Off {
                        if let Some(debug_renderer) = self.debug_renderer.as_mut() {
                            debug_renderer.begin_frame(window);

                            crate::ui::debug_ui::run_debug(
                                debug_renderer,
                                mode,
                                &velum::backend::report::snapshot_in(gpu_ctx.context_id()),
                                &self.debug_data,
                                &self.resource,
                                &mut self.cam,
                            );
                            debug_renderer.flush_into(window, &mut frame_ctx, gpu_ctx);
                        }
                    }
                }

                frame_ctx.finish(false);
            }
            Ok(None) => (),
            Err(error) => tracing::error!(%error, "Échec du rendu Velum"),
        }
    }

    fn check_gpu_errors(&self, event_loop: &winit::event_loop::ActiveEventLoop) -> bool {
        if let Some(context) = &self.gpu_ctx {
            while let Some(error) = context.take_error() {
                tracing::error!(%error, "Erreur GPU Alpha");
                if error.is_fatal() {
                    event_loop.exit();
                    return false;
                }
            }
        }
        true
    }

    fn release_gpu(&mut self) {
        // Drop resources before unregistering their GPU context.
        self.renderer = None;
        self.debug_renderer = None;
        self.asset_cache = None;
        self.shared_data = velum::RenderData::new();
        self.nodus_bridge = Default::default();
        self.map = None;
        self.ui_ctx = None;
        self.gpu_ctx = None;
        self.window = None;
        self.scale = None;
        self.input_state = Input::new();
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Winit can emit redundant resume notifications.
        if self.window.is_some() {
            return;
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        let window_attribute =
            winit::window::Window::default_attributes().with_title("Project Alpha");

        let window = match event_loop.create_window(window_attribute) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                tracing::error!("Échec de la création de la fenêtre Winit : {err}");
                event_loop.exit();
                return;
            }
        };
        let resumed_gpu = if let Some(mut gpu) = self.gpu_ctx.take() {
            gpu.replace_window(window.clone()).map(|()| gpu)
        } else {
            pollster::block_on(crate::rendering::GpuContext::new(window.clone()))
        };
        let gpu_ctx = match resumed_gpu {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::error!("Erreur lors de la création du gpu context : {e}");
                event_loop.exit();
                return;
            }
        };

        let debug_renderer = DebugRenderer::init(&window, &gpu_ctx);

        // Création de la grid temp a la fin il faut la créer via la seed envoyer par le server
        {
            let generator = utils::map::generator::Generator::new(42, 150, 100);
            let grid = generator.generate();
            let flow_field = utils::map::flow_field::FlowField::new(150, 100);

            self.debug_data.insert(flow_field);
            self.resource.insert(grid);
        }
        let mut shared_data = velum::RenderData::new();
        let mut nodus_bridge = crate::rendering::nodus_bridge::NodusBridge::default();
        let mut asset_cache = match crate::rendering::AssetCache::new(&gpu_ctx, &mut shared_data) {
            Ok(assets) => assets,
            Err(error) => {
                tracing::error!(%error, "Échec de l'initialisation des ressources Velum");
                event_loop.exit();
                return;
            }
        };

        let renderer = match self.initialize_renderer(&gpu_ctx, &mut asset_cache, &mut shared_data)
        {
            Ok(renderer) => renderer,
            Err(error) => {
                tracing::error!("{error}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let mut ui_ctx = nodus::UiContext::new(size.width as f32, size.height as f32);
        let scale = ScreenScale::new(size.width as i32, size.height as i32);

        let mut animations = AnimationManager::new();
        {
            match animations.load_from_config(
                &mut asset_cache,
                &mut shared_data,
                "assets/config/animations.json",
            ) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Echec lors du chargement des animations : {e}");
                    event_loop.exit();
                    return;
                }
            }
        }
        self.resource.insert(animations);

        let hp_material_id =
            match self.initialize_health_material(&gpu_ctx, &mut nodus_bridge, &mut shared_data) {
                Ok(id) => id,
                Err(error) => {
                    tracing::error!("{error}");
                    event_loop.exit();
                    return;
                }
            };

        // A resumed renderer must not collect commands holding old GPU/UI tokens.
        self.resource.insert(nodus::DrawCommandBuffer::new(2048));

        // Init des élements du ui des différentes scènes
        {
            hud::init_hud(&mut ui_ctx, hp_material_id, &self.resource);
            hud::init_shop(&mut ui_ctx, &self.resource);
            states::lobby::init_lobby(&mut ui_ctx, &self.resource);
        }

        let map = match TileMap::new(
            &mut asset_cache,
            &mut shared_data,
            "assets/map_texture/map_sprite_sheet.png",
        ) {
            Ok(map) => map,
            Err(e) => {
                tracing::error!("Erreur lors de la création de la map : {e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.debug_renderer = Some(debug_renderer);
        self.ui_ctx = Some(ui_ctx);
        self.scale = Some(scale);
        self.gpu_ctx = Some(gpu_ctx);
        self.asset_cache = Some(asset_cache);
        self.shared_data = shared_data;
        self.nodus_bridge = nodus_bridge;
        self.map = Some(map);
        self.last_frame = std::time::Instant::now();

        tracing::info!("Application initialisée et prête");
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Keep the native desktop renderer available for the next surface.
        let gpu = self.gpu_ctx.take();
        self.release_gpu();
        self.gpu_ctx = gpu;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.release_gpu();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
            || !self.check_gpu_errors(event_loop)
        {
            return;
        }
        if let (Some(window), Some(debug_renderer)) = (&self.window, &mut self.debug_renderer) {
            let consumed = debug_renderer.handle_event(window, &event);
            // A debug button must not also trigger an attack or movement input.
            let interactive = matches!(self.screen, core::screen::AppScreen::InGame)
                && self
                    .resource
                    .read_resource::<core::debug_state::DebugState>()
                    .mode
                    == core::debug_state::DebugMode::Interactive;
            if interactive
                && consumed
                && matches!(
                    &event,
                    winit::event::WindowEvent::MouseInput {
                        state: winit::event::ElementState::Pressed,
                        ..
                    } | winit::event::WindowEvent::KeyboardInput {
                        event: winit::event::KeyEvent {
                            state: winit::event::ElementState::Pressed,
                            ..
                        },
                        ..
                    }
                )
            {
                return;
            }
        }
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(s) => {
                let gpu_ctx = self.gpu_ctx.as_mut().unwrap();
                if let Err(error) = gpu_ctx.resize(s.width, s.height) {
                    tracing::error!(%error, "Échec du redimensionnement GPU");
                    event_loop.exit();
                    return;
                }
                if let Some(ui_ctx) = &mut self.ui_ctx {
                    ui_ctx.resize(s.width as f32, s.height as f32);
                }
                if let Some(scale) = &mut self.scale {
                    scale.resize(s.width as f32, s.height as f32);
                }
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    if event.state.is_pressed() {
                        self.input_state.pressed(key);
                    } else {
                        self.input_state.released(key);
                    }
                }
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => match button {
                winit::event::MouseButton::Left | winit::event::MouseButton::Right => {
                    if state.is_pressed() {
                        self.input_state.mouse_pressed(button);
                    } else {
                        self.input_state.mouse_release(button);
                    }
                }
                _ => (),
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.input_state
                    .set_mouse_position(position.x as f32, position.y as f32);
            }
            winit::event::WindowEvent::RedrawRequested => self.redraw(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.check_gpu_errors(event_loop) {
            return;
        }
        if self
            .input_state
            .is_pressed(winit::keyboard::KeyCode::Escape)
        {
            tracing::info!("Fermeture demandée par l'utilisateur (Touche Échap)");
            event_loop.exit();
            return;
        }

        let (Some(gpu_ctx), Some(scale)) = (&self.gpu_ctx, &self.scale) else {
            return;
        };

        let now = std::time::Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = frame_delta.as_secs_f32();

        let screen_size = gpu_ctx.size;

        {
            let runtime = self.resource.read_resource::<tokio::runtime::Runtime>();
            let mut asset_loader = self.resource.write_resource::<crate::vfx::AssetLoader>();
            runtime.block_on(asset_loader.run_request());
            let mut loads = self
                .resource
                .write_resource::<core::spell_vfx::SpellVfxLoads>();
            let mut vfx = self.resource.write_resource::<crate::vfx::VfxSystem>();
            if let Err(error) = loads.publish(&mut asset_loader, &mut vfx.registry) {
                tracing::warn!(%error, "Échec du chargement des VFX fireball");
            }
        }

        if let Some(ref mut c) = self.client {
            c.update(frame_delta);

            if matches!(
                self.screen,
                core::screen::AppScreen::MainMenu | core::screen::AppScreen::Lobby(_)
            ) {
                while let Some(msg) = c.recv_lobby_message() {
                    crate::app::states::lobby::handle_lobby_message(
                        msg,
                        &mut self.screen,
                        &mut self.is_solo,
                    );
                }
            }
        }
        match &mut self.screen {
            core::screen::AppScreen::MainMenu => {
                let action = crate::app::states::main_menu::handle_input(
                    &self.input_state,
                    &mut self.client,
                    self.client_id.0,
                );

                match action {
                    MenuAction::Solo => {
                        self.is_solo = true;
                        tracing::info!("Mode de jeu sélectionné : SOLO");
                    }
                    MenuAction::Multi => {
                        self.is_solo = false;
                        tracing::info!("Mode de jeu sélectionné : MULTI");
                    }
                    MenuAction::None => {}
                }
            }

            core::screen::AppScreen::Lobby(state) => {
                if let Some(ref mut c) = self.client {
                    crate::app::states::lobby::handle_input(&self.input_state, state, c);
                    c.flush();
                }

                crate::app::states::lobby::update(
                    self.ui_ctx.as_mut().unwrap(),
                    &self.resource,
                    state,
                );
            }

            core::screen::AppScreen::InGame => {
                let client_ok = self.client.as_mut();
                let ui_ok = self.ui_ctx.as_mut();

                if self
                    .input_state
                    .is_just_pressed(winit::keyboard::KeyCode::KeyP)
                {
                    let mut debug = self
                        .resource
                        .write_resource::<core::debug_state::DebugState>();
                    debug.cycle();
                }

                match (client_ok, ui_ok) {
                    (Some(client), Some(ui_ctx)) => {
                        let mut gui_ctx = crate::app::states::in_game::GuiContext { ui_ctx };

                        // Mise à jour logique
                        self.in_game_scene.update(
                            &mut self.resource,
                            client,
                            screen_size,
                            &mut gui_ctx,
                            &mut self.input_state,
                            scale,
                            &mut self.cam,
                            dt,
                        );
                        {
                            let debug_state = self
                                .resource
                                .read_resource::<core::debug_state::DebugState>();
                            if debug_state.mode != core::debug_state::DebugMode::Off {
                                self.debug_data.insert(debug_state.attack_box.clone());
                                self.debug_data.insert(debug_state.collider.clone());

                                // Maj flow field seulement en mod debug
                                {
                                    let grid =
                                        self.resource.read_resource::<utils::map::grid::Grid>();
                                    let new_target =
                                        self.resource.read_resource::<utils::math::Vec2>();
                                    let flow_field =
                                        self.debug_data
                                            .update_data::<utils::map::flow_field::FlowField>();
                                    if let Some(flow_field) = flow_field {
                                        if flow_field.needs_update(&grid, *new_target) {
                                            flow_field.compute(&grid, *new_target);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        tracing::error!(
                            client = self.client.is_some(),
                            ui_ctx = self.ui_ctx.is_some(),
                            asset_cache = self.asset_cache.is_some(),
                            debug_renderer = self.debug_renderer.is_some(),
                            "Impossible de rendre InGame : une ressource requise est None"
                        );
                    }
                }
            }
        }
        if let Some(ref mut ui_ctx) = self.ui_ctx {
            let hud_root = match self
                .resource
                .read_resource::<utils::ids::Register>()
                .get::<nodus::NodeId>(crate::key::hud::ROOT)
            {
                Some(id) => id,
                None => {
                    tracing::warn!("L'id {} est absent du register", crate::key::hud::ROOT);
                    return;
                }
            };
            let lobby_root = match self
                .resource
                .read_resource::<utils::ids::Register>()
                .get::<nodus::NodeId>(crate::key::lobby::ROOT)
            {
                Some(id) => id,
                None => {
                    tracing::warn!("L'id {} est absent du register", crate::key::lobby::ROOT);
                    return;
                }
            };

            let in_lobby = matches!(self.screen, core::screen::AppScreen::Lobby(_));
            let in_game = matches!(self.screen, core::screen::AppScreen::InGame);

            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: lobby_root,
                visible: in_lobby,
            });
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: hud_root, // nœud racine du HUD in-game
                visible: in_game,
            });

            ui_ctx.update(dt);
        }

        let cam_matrix = build_camera_matrix(
            self.cam.pos(),
            self.cam.shake.offset(),
            screen_size.width.max(1) as f32,
            screen_size.height.max(1) as f32,
        );
        self.resource.insert(cam_matrix);
        self.debug_data.insert(cam_matrix);
        self.debug_data.insert(screen_size);
        self.debug_data.insert(self.cam);

        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.input_state.end_frame();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.release_gpu();
    }
}

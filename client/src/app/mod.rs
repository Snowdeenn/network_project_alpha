pub mod input;
pub mod resources;
pub mod states;

use std::sync::Arc;
use utils::buffer::BufferManager;

use crate::app::input::Input;
use crate::app::resources::Resources;
use crate::app::states::in_game::InGameScene;
use crate::app::states::main_menu::MenuAction;
use crate::core;
use crate::core::client::GameNetClient;
use crate::graphic_data::asset_manager::AssetManager;
use crate::graphic_data::post_process_effect_type;
use crate::graphic_data::tile_map::TileMap;
use crate::rendering::ScreenScale;
use crate::rendering::camera::Camera;
use crate::rendering::vfx::particle::ParticlePool;
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::debug_ui::{DebugData, DebugRenderer};

fn build_camera_matrix(
    pos: utils::math::Vec2,
    shake: utils::math::Vec2,
    screen_w: f32,
    screen_h: f32,
) -> utils::math::Mat4 {
    let proj = utils::math::Mat4::orthographic_wgpu(0.0, screen_w, screen_h, 0.0, -1.0, 1.0);
    let view = utils::math::Mat4::translation(
        -pos.x + (screen_w * 0.5) + shake.x,
        -pos.y + (screen_h * 0.5) + shake.y,
        0.0,
    );
    proj.multiply(view)
}

use crate::ui::hud;

#[derive(Clone, Copy)]
pub struct ClientId(pub u64);

pub struct App {
    window: Option<Arc<winit::window::Window>>,
    gpu_ctx: Option<prism::GpuContext>,
    gpu_resources: Option<prism::GpuResources>,
    renderer: Option<prism::Renderer>,
    debug_renderer: Option<DebugRenderer>,
    ui_ctx: Option<nodus::UiContext>,
    client: Option<GameNetClient>,
    map: Option<TileMap>,
    resource: Resources,
    debug_data: DebugData,
    id_register: utils::ids::Register,
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
        resource.insert(VfxManager::new());
        resource.insert(BufferManager::with_capacity(16));
        resource.insert(ParticlePool::new());
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

        let last_frame = std::time::Instant::now();
        let id_register = utils::ids::Register::new();

        Self {
            window: None,
            renderer: None,
            debug_renderer: None,
            gpu_ctx: None,
            gpu_resources: None,
            ui_ctx: None,
            client: None,
            map: None,
            resource,
            debug_data: DebugData::default(),
            id_register,
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

    pub fn renderer(&self) -> &prism::Renderer {
        self.renderer.as_ref().unwrap()
    }

    pub fn renderer_mut(&mut self) -> &mut prism::Renderer {
        self.renderer.as_mut().unwrap()
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
        let gpu_ctx = match pollster::block_on(prism::GpuContext::new(window.clone())) {
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
        let mut gpu_resources = prism::GpuResources::new(&gpu_ctx);

        // Chargement des shaders par defaut
        // On exit la loop si le chargement échoue parce que si les shaders par defaut
        // ne sont pas charger le client n'affichera rien
        let default_vert_id = match gpu_resources
            .load_shader(&gpu_ctx, "client/src/graphic_data/shader/default.vert.wgsl")
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        };
        self.id_register
            .insert(crate::key::shader::DEFAULT_VERTEX, default_vert_id);

        let default_frag_id = match gpu_resources
            .load_shader(&gpu_ctx, "client/src/graphic_data/shader/default.frag.wgsl")
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        };
        self.id_register
            .insert(crate::key::shader::DEFAULT_FRAGMENT, default_frag_id);

        let post_vert_id = match gpu_resources.load_shader(
            &gpu_ctx,
            "client/src/graphic_data/shader/default_post_process.vert.wgsl",
        ) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        };
        self.id_register
            .insert(crate::key::post::DEFAULT_POST_VERTEX, post_vert_id);

        let post_frag_id = match gpu_resources.load_shader(
            &gpu_ctx,
            "client/src/graphic_data/shader/default_post_process.frag.wgsl",
        ) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        };
        self.id_register
            .insert(crate::key::post::DEFAULT_POST_FRAGMENT, post_frag_id);

        match gpu_resources.load_shader(
            &gpu_ctx,
            "client/src/graphic_data/shader/default_textured.vert.wgsl",
        ) {
            Ok(id) => self
                .id_register
                .insert(crate::key::shader::TEXTURED_VERTEX, id),
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        }
        match gpu_resources.load_shader(
            &gpu_ctx,
            "client/src/graphic_data/shader/default_textured.frag.wgsl",
        ) {
            Ok(id) => self
                .id_register
                .insert(crate::key::shader::TEXTURED_FRAGMENT, id),
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        }
        match gpu_resources.load_shader(
            &gpu_ctx,
            "client/src/graphic_data/shader/hit_flash_effect.frag.wgsl",
        ) {
            Ok(id) => self
                .id_register
                .insert(crate::key::post::HIT_FLASH_FRAG, id),
            Err(e) => {
                tracing::error!("Erreur lors du chargement du shader : {e}");
                event_loop.exit();
                return;
            }
        }
        let text_vert_id = self
            .id_register
            .get::<utils::ids::ShaderId>(crate::key::shader::TEXTURED_VERTEX)
            .unwrap();
        let text_frag_id = self
            .id_register
            .get::<utils::ids::ShaderId>(crate::key::shader::TEXTURED_FRAGMENT)
            .unwrap();
        let mut renderer = match prism::Renderer::new(
            &gpu_ctx,
            &mut gpu_resources,
            default_vert_id,
            default_frag_id,
            text_vert_id,
            text_frag_id,
        ) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!("Échec de l'initialisation du renderer Prism : {err}");
                event_loop.exit();
                return;
            }
        };

        // Ajout des RenderPass du post process
        {
            // Pass par defaut pass throught
            let _default_pass_id = match renderer.add_post_process_pass::<()>(
                &gpu_ctx,
                &gpu_resources,
                post_vert_id,
                post_frag_id,
                None,
            ) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Erreur lors de la création de la Post Process Pass: {e}");
                    event_loop.exit();
                    return;
                }
            };

            let hit_flash_shader_id = self
                .id_register
                .get::<utils::ids::ShaderId>(crate::key::post::HIT_FLASH_FRAG)
                .expect("Le hit flash id devrait être la");

            let uniform = post_process_effect_type::HitFlashUniform { intensity: 0.5 };
            let hit_flash_id = match renderer.add_post_process_pass(
                &gpu_ctx,
                &gpu_resources,
                post_vert_id,
                hit_flash_shader_id,
                Some(uniform),
            ) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Erreur lors de la création de la Post Process Pass: {e}");
                    event_loop.exit();
                    return;
                }
            };
            renderer.disable_post_process_pass(hit_flash_id);
            let hit_flash = post_process_effect_type::HitFlashEffect {
                id: hit_flash_id,
                timer: 0.0, // Init à 0 parce que le joueur n'est pas hit
                total_duration: 0.2,
                intensity: uniform.intensity,
            };
            self.resource.insert(hit_flash);
        }

        let size = window.inner_size();
        let mut ui_ctx = nodus::UiContext::new(size.width as f32, size.height as f32);
        let scale = ScreenScale::new(size.width as i32, size.height as i32);

        let mut asset_manager = AssetManager::new();
        {
            match asset_manager.load_animations(
                &gpu_ctx,
                &mut gpu_resources,
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
        self.resource.insert(asset_manager);

        let hp_material_id = {
            let vert_id = self
                .id_register
                .get::<utils::ids::ShaderId>(crate::key::shader::TEXTURED_VERTEX)
                .unwrap();
            let frag_id = gpu_resources
                .load_shader(
                    &gpu_ctx,
                    "client/src/graphic_data/shader/progress_bar.frag.wgsl",
                )
                .unwrap();
            self.id_register.insert("shader/progress_bar_frag", frag_id);

            // Créer la pipeline pour ce matériau
            let pipeline = match renderer.create_pipeline(
                &gpu_ctx,
                &gpu_resources,
                prism::PipelineKey {
                    vertex_shader: vert_id,
                    fragment_shader: frag_id,
                    blend_mode: prism::BlendMode::Alpha,
                    vertex_format: prism::VertexFormat::Pos2UvColor,
                    bind_groups: &prism::MATERIAL_BIND_GROUP,
                },
            ) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Impossible de créer la pipeline : {e}");
                    event_loop.exit();
                    return;
                }
            };

            gpu_resources.create_material(
                pipeline,
                vec![], // pas de bind groups custom supplémentaires — les uniforms passent par le scratch buffer
                std::mem::size_of::<f32>(), // uniform_size : un f32 (le ratio)
            )
        };
        self.id_register
            .insert(crate::key::material::HP_MATERIAL, hp_material_id);

        // Init des élements du ui des différentes scènes
        {
            hud::init_hud(&mut ui_ctx, hp_material_id, &mut self.id_register);
            hud::init_shop(&mut ui_ctx, &mut self.id_register);
            states::lobby::init_lobby(&mut ui_ctx, &mut self.id_register);
        }

        let map = match TileMap::new(&mut gpu_resources, &gpu_ctx) {
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
        self.gpu_resources = Some(gpu_resources);
        self.map = Some(map);

        tracing::info!("Application initialisée et prête");
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let (Some(window), Some(debug_renderer)) = (&self.window, &mut self.debug_renderer) {
            let _ = debug_renderer.handle_event(window, &event);
        }
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(s) => {
                let gpu_ctx = self.gpu_ctx.as_mut().unwrap();
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(gpu_ctx, s.width, s.height);
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
            winit::event::WindowEvent::RedrawRequested => {
                let gpu_ctx = self.gpu_ctx.as_mut().unwrap();
                let gpu_resources = self.gpu_resources.as_mut().unwrap();
                let window = self.window.as_ref().unwrap();
                if let Some(renderer) = &mut self.renderer {
                    if let Some(frame) = renderer.frame_manager().pop() {
                        if let Some(mut frame_ctx) = renderer.render(gpu_ctx, gpu_resources, frame)
                        {
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
                                            &self.debug_data,
                                            &self.resource,
                                        );
                                        debug_renderer.flush_into(window, &mut frame_ctx, gpu_ctx);
                                    }
                                }
                            }

                            frame_ctx.present(gpu_ctx);
                        }
                    } else {
                        tracing::trace!("Aucune frame prête lors du RedrawRequested");
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self
            .input_state
            .is_pressed(winit::keyboard::KeyCode::Escape)
        {
            tracing::info!("Fermeture demandée par l'utilisateur (Touche Échap)");
            event_loop.exit();
            return;
        }

        let (Some(gpu_ctx), Some(scale), Some(renderer)) =
            (&mut self.gpu_ctx, &self.scale, &mut self.renderer)
        else {
            return;
        };

        let now = std::time::Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = frame_delta.as_secs_f32();

        let screen_size = gpu_ctx.size;
        let mut frame = prism::Frame::new();
        frame.camera_pos = self.cam.pos();
        frame.cam_shake_offset = self.cam.shake.offset();
        let cam_matrix = build_camera_matrix(
            self.cam.pos(),
            self.cam.shake.offset(),
            screen_size.width as f32,
            screen_size.height as f32,
        );
        self.resource.insert(cam_matrix);
        self.debug_data.insert(cam_matrix);
        self.debug_data.insert(screen_size);
        self.debug_data.insert(self.cam);

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
            if matches!(self.screen, core::screen::AppScreen::InGame) {
                renderer.frame_manager().clear();
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

                // Rendu Main Menu
                match &self.client {
                    None => crate::app::states::main_menu::render(&mut frame, scale),
                    Some(_) => crate::app::states::main_menu::render_connecting(&mut frame, scale),
                }
            }

            core::screen::AppScreen::Lobby(state) => {
                if let Some(ref mut c) = self.client {
                    crate::app::states::lobby::handle_input(&self.input_state, state, c);
                    c.flush();
                }

                // Rendu Lobby
                crate::app::states::lobby::update(
                    &mut self.ui_ctx.as_mut().unwrap(),
                    &self.id_register,
                    state,
                );
            }

            core::screen::AppScreen::InGame => {
                let client_ok = self.client.as_mut();
                let ui_ok = self.ui_ctx.as_mut();
                let gpu_resources_ok = self.gpu_resources.as_mut();

                if self
                    .input_state
                    .is_just_pressed(winit::keyboard::KeyCode::KeyP)
                {
                    let mut debug = self
                        .resource
                        .write_resource::<core::debug_state::DebugState>();
                    debug.cycle();
                }

                // Activation des RenderPass post process
                {
                    let hit_flash = self
                        .resource
                        .read_resource::<post_process_effect_type::HitFlashEffect>();
                    renderer.enable_post_process_pass(hit_flash.id);
                }

                match (client_ok, ui_ok, gpu_resources_ok) {
                    (Some(client), Some(ui_ctx), Some(gpu_resources)) => {
                        let mut gui_ctx = crate::app::states::in_game::GuiContext {
                            ui_ctx,
                            gpu_resources,
                            ids: &self.id_register,
                        };

                        // Mise à jour logique
                        self.in_game_scene.update(
                            &mut self.resource,
                            client,
                            screen_size,
                            &mut gui_ctx,
                            &self.input_state,
                            scale,
                            &mut self.cam,
                            dt,
                        );
                        // Rendu de la scène InGame
                        self.in_game_scene
                            .render(&mut frame, &mut self.resource, dt);

                        if let Some(map) = &self.map {
                            map.draw(
                                &self.resource,
                                gpu_resources,
                                &mut frame,
                                screen_size.width as f32,
                                screen_size.height as f32,
                            );
                        }

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

                        {
                            let hit_flash = self
                                .resource
                                .read_resource::<post_process_effect_type::HitFlashEffect>();
                            renderer.write_post_process_uniform(
                                gpu_ctx,
                                *hit_flash.id,
                                hit_flash.intensity,
                            );
                        }
                    }
                    _ => {
                        tracing::error!(
                            client = self.client.is_some(),
                            ui_ctx = self.ui_ctx.is_some(),
                            gpu_resources = self.gpu_resources.is_some(),
                            debug_renderer = self.debug_renderer.is_some(),
                            "Impossible de rendre InGame : une ressource requise est None"
                        );
                    }
                }
            }
        }
        if let Some(ref mut ui_ctx) = self.ui_ctx {
            let hud_root = match self.id_register.get::<nodus::NodeId>(crate::key::hud::ROOT) {
                Some(id) => id,
                None => {
                    tracing::warn!("L'id {} est absent du register", crate::key::hud::ROOT);
                    return;
                }
            };
            let lobby_root = match self
                .id_register
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
            let mut buf = self.resource.write_resource::<nodus::DrawCommandBuffer>();
            ui_ctx.update(dt);
            hud::prepare_hud(&mut frame, ui_ctx, &mut buf);
        }

        if let Err(rejected_frame) = renderer.frame_manager().push(frame) {
            tracing::warn!("Frame rejetée : la file d'attente du FrameManager est pleine");
            let _ = rejected_frame;
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.input_state.end_frame();
    }
}

pub mod input;
pub mod resources;
pub mod states;

use std::sync::Arc;
use utils::buffer::BufferManager;

use crate::app::input::Input;
use crate::app::resources::Resources;
use crate::app::states::in_game::{InGameIds, InGameScene};
use crate::app::states::main_menu::MenuAction;
use crate::core::client::GameNetClient;
use crate::core::event::AppScreen;
use crate::graphic_data::asset_manager::AssetManager;
use crate::rendering::ScreenScale;
use crate::rendering::camera::Camera;
use crate::rendering::vfx::particle::ParticlePool;
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::hud;

pub struct App {
    window: Option<Arc<winit::window::Window>>,
    renderer: Option<prism::Renderer>,
    ui_ctx: Option<ui::UiContext>,
    client: Option<GameNetClient>,
    resource: Resources,
    client_id: u64,
    in_game_scene: InGameScene,
    screen: AppScreen,
    in_game_ids: Option<InGameIds>,
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

        let last_frame = std::time::Instant::now();

        Self {
            window: None,
            renderer: None,
            ui_ctx: None,
            client: None,
            resource,
            client_id: rand::random::<u64>(),
            in_game_scene: InGameScene::default(),
            screen: AppScreen::MainMenu,
            in_game_ids: None,
            is_solo: false,
            input_state: Input::new(),
            last_frame,
            scale: None,
            cam: Camera::default(),
        }
    }

    pub fn renderer(&self) -> Option<&prism::Renderer> {
        self.renderer.as_ref()
    }

    pub fn renderer_mut(&mut self) -> Option<&mut prism::Renderer> {
        self.renderer.as_mut()
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

        let mut renderer = match prism::Renderer::new(
            window.clone(),
            "client/src/graphic_data/shader/default.vert.wgsl",
            "client/src/graphic_data/shader/default.frag.wgsl",
            "client/src/graphic_data/shader/default_post_process.vert.wgsl",
            "client/src/graphic_data/shader/default_post_process.frag.wgsl",
        ) {
            Ok(r) => r,
            Err(err) => {
                tracing::error!("Échec de l'initialisation du renderer Prism : {err}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let mut ui_ctx = ui::UiContext::new(size.width as f32, size.height as f32);
        let scale = ScreenScale::new(size.width as i32, size.height as i32);

        let mut asset_manager = AssetManager::new();
        {
            let (ctx, textures) = renderer.ctx_and_textures_mut();
            match  asset_manager.load_animations(ctx, textures, "assets/config/animations.json") {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Echec lors du chargement des animations : {e}");
                    event_loop.exit();
                    return;
                }
            }
        }
        self.resource.insert(asset_manager);

        // Chargement du shader de barre de progression
        let sh_id =
            match renderer.load_shader("client/src/graphic_data/shader/progress_bar.frag.wgsl") {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!("Impossible de charger le shader progress_bar : {err}");
                    event_loop.exit();
                    return;
                }
            };

        let hud_node_id = hud::init_hud(&mut ui_ctx, sh_id);
        let shop_id = hud::init_shop(&mut ui_ctx);

        self.in_game_ids = Some(InGameIds {
            shop: shop_id,
            hud: hud_node_id,
            shader: sh_id,
        });

        // Chargement des shaders texturés pour le World
        match (
            renderer.load_shader("client/src/graphic_data/shader/default_textured.vert.wgsl"),
            renderer.load_shader("client/src/graphic_data/shader/default_textured.frag.wgsl"),
        ) {
            (Ok(vs), Ok(fs)) => {
                if let Err(err) = renderer.set_world_shaders(vs, fs) {
                    tracing::error!("Échec du paramétrage des shaders World : {err}");
                }
            }
            (Err(err), _) | (_, Err(err)) => {
                tracing::error!("Impossible de charger les shaders texturés : {err}");
            }
        }

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui_ctx = Some(ui_ctx);
        self.scale = Some(scale);

        tracing::info!("Application initialisée et prête");
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(s) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(s.width, s.height);
                }
                if let Some(ui_ctx) = &mut self.ui_ctx {
                    ui_ctx.resize(s.width as f32, s.height as f32);
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
                if let Some(renderer) = &mut self.renderer {
                    if let Some(frame) = renderer.frame_manager().pop() {
                        renderer.render(frame);
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

        let (Some(renderer), Some(scale)) = (&mut self.renderer, &self.scale) else {
            return;
        };

        let now = std::time::Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = frame_delta.as_secs_f32();

        let mut frame = prism::Frame::new();
        frame.camera_pos = self.cam.pos();
        frame.cam_shake_offset = self.cam.shake.offset();

        let screen_size = renderer.screen_size();

        if let Some(ref mut c) = self.client {
            c.update(frame_delta);

            if matches!(self.screen, AppScreen::MainMenu | AppScreen::Lobby(_)) {
                while let Some(msg) = c.recv_lobby_message() {
                    crate::app::states::lobby::handle_lobby_message(
                        msg,
                        &mut self.screen,
                        &mut self.is_solo,
                    );
                }
            }
            if matches!(self.screen, AppScreen::InGame(_)) {
                tracing::info!("frame manager clear");
                renderer.frame_manager().clear();
            }
        }
        match &mut self.screen {
            AppScreen::MainMenu => {
                let action = crate::app::states::main_menu::handle_input(
                    &self.input_state,
                    &mut self.client,
                    self.client_id,
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

            AppScreen::Lobby(state) => {
                if let Some(ref mut c) = self.client {
                    crate::app::states::lobby::handle_input(&self.input_state, state, c);
                    c.flush();
                }

                // Rendu Lobby
                crate::app::states::lobby::render(&mut frame, state, scale);
            }

            AppScreen::InGame(client_state) => {
                let client_ok = self.client.as_mut();
                let ui_ok = self.ui_ctx.as_mut();
                let ids_ok = self.in_game_ids.as_ref();

                match (client_ok, ui_ok, ids_ok) {
                    (Some(client), Some(ui_ctx), Some(in_game_ids)) => {
                        let mut gui_ctx = crate::app::states::in_game::GuiContext {
                            ui_ctx,
                            shader_manager: renderer.shader_mut(),
                            ids: in_game_ids,
                        };

                        // Mise à jour logique
                        self.in_game_scene.update(
                            &mut self.resource,
                            client,
                            screen_size,
                            client_state,
                            &mut gui_ctx,
                            &self.input_state,
                            scale,
                            &mut self.cam,
                            dt,
                        );

                        // Rendu de la scène InGame
                        self.in_game_scene
                            .render(&mut frame, client_state, &mut self.resource, dt);
                    }
                    _ => {
                        tracing::error!(
                            client = self.client.is_some(),
                            ui_ctx = self.ui_ctx.is_some(),
                            in_game_ids = self.in_game_ids.is_some(),
                            "Impossible de rendre InGame : une ressource requise est None"
                        );
                    }
                }
            }
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

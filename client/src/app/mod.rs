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
        let window = event_loop.create_window(window_attribute).unwrap();
        let window = Arc::new(window);
        let mut renderer = prism::Renderer::new(
            window.clone(),
            "client/src/graphic_data/shader/default.vert.wgsl",
            "client/src/graphic_data/shader/default.frag.wgsl",
            "client/src/graphic_data/shader/default_post_process.vert.wgsl",
            "client/src/graphic_data/shader/default_post_process.frag.wgsl",
        );
        let ui_ctx = ui::UiContext::new(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        let scale = ScreenScale::new(
            window.inner_size().width as i32,
            window.inner_size().height as i32,
        );
        let mut asset_manager = AssetManager::new();
        {
            let (ctx, textures) = renderer.ctx_and_textures_mut();
            asset_manager.load_animations(ctx, textures, "assets/config/animations.json");
        }
        self.resource.insert(asset_manager);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui_ctx = Some(ui_ctx);
        self.scale = Some(scale);
        let sh_id = self
            .renderer
            .as_mut()
            .unwrap()
            .load_shader("client/src/graphic_data/shader/progress_bar.frag.wgsl")
            .unwrap();

        let hud_node_id = hud::init_hud(&mut self.ui_ctx.as_mut().unwrap(), sh_id);
        let shop_id = hud::init_shop(&mut self.ui_ctx.as_mut().unwrap());
        self.in_game_ids = Some(InGameIds {
            shop: shop_id,
            hud: hud_node_id,
            shader: sh_id,
        });
        let tex_vert_shader_id = self
            .renderer_mut()
            .load_shader("client/src/graphic_data/shader/default_textured.vert.wgsl")
            .unwrap();
        let tex_frag_shader_id = self
            .renderer_mut()
            .load_shader("client/src/graphic_data/shader/default_textured.frag.wgsl")
            .unwrap();
        self.renderer_mut()
            .set_world_shaders(tex_vert_shader_id, tex_frag_shader_id);
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
                self.renderer.as_mut().unwrap().resize(s.width, s.height);
                self.ui_ctx
                    .as_mut()
                    .unwrap()
                    .resize(s.width as f32, s.height as f32);
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                match (event.physical_key, event.state) {
                    (
                        winit::keyboard::PhysicalKey::Code(key),
                        winit::event::ElementState::Pressed,
                    ) => {
                        self.input_state.pressed(key);
                    }
                    (
                        winit::keyboard::PhysicalKey::Code(key),
                        winit::event::ElementState::Released,
                    ) => {
                        self.input_state.released(key);
                    }
                    (winit::keyboard::PhysicalKey::Unidentified(_), _) => (),
                }
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (winit::event::MouseButton::Left, winit::event::ElementState::Pressed) => {
                        self.input_state
                            .mouse_pressed(winit::event::MouseButton::Left);
                    }
                    (winit::event::MouseButton::Left, winit::event::ElementState::Released) => {
                        self.input_state
                            .mouse_release(winit::event::MouseButton::Left);
                    }
                    (winit::event::MouseButton::Right, winit::event::ElementState::Pressed) => {
                        self.input_state
                            .mouse_pressed(winit::event::MouseButton::Right);
                    }
                    (winit::event::MouseButton::Right, winit::event::ElementState::Released) => {
                        self.input_state
                            .mouse_release(winit::event::MouseButton::Right);
                    }
                    _ => (),
                };
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.input_state
                    .set_mouse_position(position.x as f32, position.y as f32);
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(frame) = self.renderer.as_mut().unwrap().frame_manager().pop() {
                    self.renderer.as_mut().unwrap().render(frame);
                } else {
                    // En mode Poll, il peut arriver qu'une frame ne soit pas encore prête
                    // On ne bloque plus la boucle ici.
                    eprintln!("Frame non ready");
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = std::time::Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = frame_delta.as_secs_f32();

        let mut frame = prism::Frame::new();
        frame.camera_pos = self.cam.pos();
        frame.cam_shake_offset = self.cam.shake.offset();

        let screen_size = self.renderer.as_ref().unwrap().screen_size();
        if let Some(ref mut c) = self.client {
            c.update(frame_delta);

            while let Some(msg) = c.recv_lobby_message() {
                crate::app::states::lobby::handle_lobby_message(
                    msg,
                    &mut self.screen,
                    &mut self.is_solo,
                );
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
                        println!("SOLO");
                    }
                    MenuAction::Multi => {
                        self.is_solo = false;
                        println!("MULTI");
                    }
                    MenuAction::None => {}
                }

                // Rendu Main Menu
                match &self.client {
                    None => crate::app::states::main_menu::render(&mut frame, &self.scale.unwrap()),
                    Some(_) => crate::app::states::main_menu::render_connecting(
                        &mut frame,
                        &self.scale.unwrap(),
                    ),
                }
            }

            AppScreen::Lobby(state) => {
                if let Some(ref mut c) = self.client {
                    crate::app::states::lobby::handle_input(&self.input_state, state, c);
                    c.flush();
                }

                // Rendu Lobby
                crate::app::states::lobby::render(&mut frame, state, &self.scale.unwrap());
            }

            AppScreen::InGame(client_state) => {
                let client = self.client.as_mut().expect("InGame sans client réseau");

                let mut gui_ctx = crate::app::states::in_game::GuiContext {
                    ui_ctx: &mut self.ui_ctx.as_mut().unwrap(),
                    shader_manager: self.renderer.as_mut().unwrap().shader_mut(),
                    ids: &self.in_game_ids.as_ref().unwrap(),
                };

                // Mise à jour logique de la partie
                self.in_game_scene.update(
                    &mut self.resource,
                    client,
                    screen_size,
                    client_state,
                    &mut gui_ctx,
                    &self.input_state,
                    &self.scale.unwrap(),
                    &mut self.cam,
                    dt,
                );

                // Rendu de la frame
                self.in_game_scene
                    .render(&mut frame, client_state, &mut self.resource, dt);
            }
        }
        self.renderer.as_ref().unwrap().frame_manager().push(frame);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.input_state.end_frame();
        if self
            .input_state
            .is_pressed(winit::keyboard::KeyCode::Escape)
        {
            std::process::exit(0);
        }
    }
}

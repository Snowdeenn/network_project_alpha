pub mod input;
pub mod resources;
pub mod states;
use std::sync::Arc;

use utils::buffer::BufferManager;

use crate::app::input::Input;
use crate::app::resources::Resources;
use crate::app::states::in_game::{InGameIds, InGameScene};
use crate::core::event::AppScreen;
use crate::graphic_data::asset_manager::AssetManager;
use crate::rendering::ScreenScale;
use crate::rendering::vfx::particle::ParticlePool;
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::hud;
use crate::core::client::GameNetClient;
use crate::app::states::main_menu::MenuAction;

pub struct App {
    window: Option<Arc<winit::window::Window>>,
    renderer: Option<prism::Renderer>,
    ui_ctx: Option<ui::UiContext>,
    client: Option<GameNetClient>,
    asset_manager: Option<AssetManager>,
    resource: Resources,
    event_loop: winit::event_loop::EventLoop<()>,
    client_id: u64,
    in_game_scene: InGameScene,
    screen: AppScreen,
    in_game_ids: Option<InGameIds>,
    is_solo: bool,
    input_state: Input,
    last_frame: std::time::Instant,
    scale: Option<ScreenScale>,
}

impl App {
    pub fn new(event_loop: winit::event_loop::EventLoop<()>) -> Self {
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
            asset_manager: None,
            resource,
            event_loop,
            client_id: rand::random::<u64>(),
            in_game_scene: InGameScene::default(),
            screen: AppScreen::MainMenu,
            in_game_ids: None,
            is_solo: false,
            input_state: Input::new(),
            last_frame,
            scale: None,
        }
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attribute =
            winit::window::Window::default_attributes().with_title("Project Alpha");
        let window = event_loop.create_window(window_attribute).unwrap();
        let window = Arc::new(window);
        let renderer = prism::Renderer::new(
            window.clone(),
            "../graphic_data/shader/default.vert.wgsl",
            "../graphic_data/shader/default.frag.wgsl",
            "../graphic_data/shader/default_post.vert.wgsl",
            "../graphic_data/shader/default_post.frag.wgsl",
        );
        let ui_ctx = ui::UiContext::new(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        let scale = ScreenScale::new(
            window.inner_size().width as i32, 
            window.inner_size().height as i32
        );
        let asset_manager = AssetManager::new();
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui_ctx = Some(ui_ctx);
        self.asset_manager = Some(asset_manager);
        self.scale = Some(scale);
        let sh_id = self
            .renderer
            .as_mut()
            .unwrap()
            .shader_mut()
            .load(
                &self.renderer.unwrap().ctx(),
                "../graphic_data/shader/progress_bar.frag.wgsl",
            )
            .unwrap();

        let hud_node_id = hud::init_hud(&mut self.ui_ctx.unwrap(), sh_id);
        let shop_id = hud::init_shop(&mut self.ui_ctx.unwrap());
        self.in_game_ids = Some(InGameIds {
            shop: shop_id,
            hud: hud_node_id,
            shader: sh_id,
        })
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
                let frame = self
                    .renderer
                    .as_mut()
                    .unwrap()
                    .frame_manager()
                    .pop()
                    .unwrap();
                self.renderer.as_mut().unwrap().render(frame);
                self.input_state.end_frame();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = std::time::Instant::now();
        let frame_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        let dt = frame_delta.as_secs_f32();

        if let Some(ref mut c) = self.client {
            c.update(frame_delta);

            while let Some(msg) = c.recv_lobby_message() {
                crate::app::states::lobby::handle_lobby_message(msg, &mut self.screen, &mut self.is_solo);
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
                let mut d = self.renderer.rl.begin_drawing(&self.renderer.thread);
                match &self.client {
                    None => crate::app::states::main_menu::render(&mut d, &self.renderer.screen_scale),
                    Some(_) => crate::app::states::main_menu::render_connecting(
                        &mut d,
                        &self.renderer.screen_scale,
                    ),
                }
            }

            AppScreen::Lobby(state) => {
                if let Some(ref mut c) = self.client {
                    crate::app::states::lobby::handle_input(&self.input_state, state, c);
                    c.flush();
                }

                // Rendu Lobby
                let mut d = self.renderer.rl.begin_drawing(&self.renderer.thread);
                crate::app::states::lobby::render(&mut d, state, &self.renderer.screen_scale);
            }

            AppScreen::InGame(client_state) => {
                let client = self.client.as_mut().expect("InGame sans client réseau");

                let mut gui_ctx = crate::app::states::in_game::GuiContext {
                    ui_ctx: &mut self.ui_ctx.unwrap(),
                    shader_manager: &mut self.shader_manager,
                    ids: &self.ingame_ids,
                };

                // Mise à jour logique de la partie
                self.ingame_scene.update(
                    &mut self.resource,
                    client,
                    &mut self.renderer,
                    client_state,
                    &mut gui_ctx,
                    dt,
                );

                // Rendu de la frame
                self.ingame_scene.render(
                    &mut self.renderer,
                    client_state,
                    &mut crate::rendering::types::RenderContext {
                        buffer: &mut self.draw_buffer,
                        shader_manager: &mut self.shader_manager,
                        ui_ctx: &mut self.ui_ctx.unwrap(),
                    },
                    &mut self.resource,
                );
            }
        }

        if self.input_state.is_pressed(winit::keyboard::KeyCode::Escape) {
            std::process::exit(0);
        }
    }
}

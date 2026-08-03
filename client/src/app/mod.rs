pub mod resources;
pub mod input;
pub mod states;
use std::sync::Arc;

use utils::buffer::BufferManager;

use crate::app::resources::Resources;
use crate::app::input::Input;
use crate::app::states::in_game::{InGameIds, InGameScene};
use crate::core::event::AppScreen;
use crate::graphic_data::asset_manager::AssetManager;
use crate::rendering::vfx::particle::ParticlePool;
use crate::rendering::vfx::vfx_manager::VfxManager;
pub struct App {
    window: Option<Arc<winit::window::Window>>,
    renderer: Option<prism::Renderer>,
    ui_ctx: Option<ui::UiContext>,
    client: Option<GameNetClient>,
    asset_manager: Option<AssetManager>,
    resource: Resources,
    event_loop: winit::EventLoop,
    client_id: u64,
    in_game_scene: InGameScene,
    screen: AppScreen,
    in_game_ids: Option<InGameIds>,
    is_solo: bool,
    input_state: Input,
}

impl App {
    pub fn new(event_loop: winit::EventLoop) -> Self {
        let mut resource = Resources::new();
        resource.insert(VfxManager::new());
        resource.insert(BufferManager::with_capacity(16));
        resource.insert(ParticlePool::new());

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
        let asset_manager = AssetManager::new();
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui_ctx = Some(ui_ctx);
        self.asset_manager = Some(asset_manager);
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
            },
            winit::event::WindowEvent::Resized(s) => {
                self.renderer.as_mut().unwrap().resize(s.width, s.height);
                self.ui_ctx
                    .as_mut()
                    .unwrap()
                    .resize(s.width as f32, s.height as f32);
            },
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
            },
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (winit::event::MouseButton::Left, winit::event::ElementState::Pressed) => {
                        self.input_state.mouse_pressed(winit::event::MouseButton::Left);
                    }
                    (winit::event::MouseButton::Left, winit::event::ElementState::Released) => {
                        self.input_state.mouse_release(winit::event::MouseButton::Left);
                    }
                    (winit::event::MouseButton::Right, winit::event::ElementState::Pressed) => {
                        self.input_state.mouse_pressed(winit::event::MouseButton::Right);
                    }
                    (winit::event::MouseButton::Right, winit::event::ElementState::Released) => {
                        self.input_state.mouse_release(winit::event::MouseButton::Right);
                    }
                    _ => (),
                };
            },
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
            },
            _ => (),
        }
    }
}

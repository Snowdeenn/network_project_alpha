// src/app/mod.rs
pub mod input;
pub mod resources;
pub mod states;

use raylib::ffi::KeyboardKey;
use std::time::Instant;

use ui::context::UiContext;
use ui::draw::DrawCommandBuffer;

use crate::app;
use crate::app::resources::Resources;
use crate::app::states::in_game::{GuiContext, InGameIds, InGameScene};
use crate::app::states::main_menu::MenuAction;
use crate::core::client::GameNetClient;
use crate::core::event::AppScreen;
use crate::graphic_data::asset_manager::AssetManager;
use crate::rendering::Renderer;
use crate::rendering::shader_manager::ShaderManager;
use crate::rendering::types::RenderContext;
use crate::rendering::vfx::particle::ParticlePool;
use crate::rendering::vfx::vfx_manager::VfxManager;
use crate::ui::hud;
use shared::buffer::BufferManager;

pub struct App {
    client_id: u64,
    renderer: Renderer,
    client: Option<GameNetClient>,
    screen: AppScreen,
    draw_buffer: DrawCommandBuffer,
    shader_manager: ShaderManager,
    ui_ctx: UiContext,
    last_frame: Instant,
    is_solo: bool,
    ingame_scene: InGameScene,
    ingame_ids: InGameIds,
    resource: Resources,
}

impl App {
    pub fn new() -> Self {
        let client_id = rand::random::<u64>();
        let mut renderer = Renderer::new(1280, 720);
        let client: Option<GameNetClient> = None;
        let screen = AppScreen::MainMenu;
        let draw_buffer = DrawCommandBuffer::new(4046);
        let mut ui_ctx = UiContext::new(renderer.screen_w as f32, renderer.screen_h as f32);
        let mut shader_manager = ShaderManager::new();
        let mut asset_manager = AssetManager::new();
        let mut resource = Resources::new();

        let last_frame = Instant::now();
        let is_solo = false;
        asset_manager.load_animations(
            &mut renderer.rl,
            &renderer.thread,
            "assets/config/animations.json",
        );

        // --- Resource insertion ---
        {
            resource.insert(asset_manager);
            resource.insert(VfxManager::new());
            resource.insert(ParticlePool::new());
            resource.insert(BufferManager::with_capacity(16));
        }

        // Scène de jeu
        let ingame_scene = InGameScene::default();

        // Shaders & UI HUD
        let sh_pr_bar = include_str!("../graphic_data/shader/progress_bar.frag");
        let raw_sh = renderer
            .rl
            .load_shader_from_memory(&renderer.thread, None, Some(sh_pr_bar));
        let sh_pr_id = shader_manager.register(raw_sh);

        let hud_node_id = hud::init_hud(&mut ui_ctx, sh_pr_id);
        let shop_ids = hud::init_shop(&mut ui_ctx);

        let ingame_ids = InGameIds {
                shop: shop_ids,
                hud: hud_node_id,
                shader: sh_pr_id,
            };

        Self {
            client_id,
            renderer,
            client,
            screen,
            draw_buffer,
            shader_manager,
            ui_ctx,
            last_frame,
            is_solo,
            ingame_scene,
            ingame_ids,
            resource,
        }
    }

    pub fn run(&mut self) {
        while !self.renderer.rl.window_should_close() {
            let frame_delta = self.last_frame.elapsed();
            self.last_frame = Instant::now();
            let dt = self.renderer.rl.get_frame_time();

            if let Some(ref mut c) = self.client {
                c.update(frame_delta);

                while let Some(msg) = c.recv_lobby_message() {
                    app::states::lobby::handle_lobby_message(
                        msg,
                        &mut self.screen,
                        &mut self.is_solo,
                    );
                }
            }

            match &mut self.screen {
                AppScreen::MainMenu => {
                    let action = app::states::main_menu::handle_input(
                        &self.renderer.rl,
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
                        None => app::states::main_menu::render(&mut d, &self.renderer.screen_scale),
                        Some(_) => app::states::main_menu::render_connecting(
                            &mut d,
                            &self.renderer.screen_scale,
                        ),
                    }
                }

                AppScreen::Lobby(state) => {
                    if let Some(ref mut c) = self.client {
                        app::states::lobby::handle_input(&self.renderer.rl, state, c);
                        c.flush();
                    }

                    // Rendu Lobby
                    let mut d = self.renderer.rl.begin_drawing(&self.renderer.thread);
                    app::states::lobby::render(&mut d, state, &self.renderer.screen_scale);
                }

                AppScreen::InGame(client_state) => {
                    let client = self.client.as_mut().expect("InGame sans client réseau");

                    let mut gui_ctx = GuiContext {
                        ui_ctx: &mut self.ui_ctx,
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
                        &mut RenderContext {
                            buffer: &mut self.draw_buffer,
                            shader_manager: &mut self.shader_manager,
                            ui_ctx: &mut self.ui_ctx,
                        },
                        &mut self.resource,
                    );
                }
            }

            if self.renderer.rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                std::process::exit(0);
            }
        }
    }
}

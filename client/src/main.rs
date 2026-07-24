mod camera;
mod config;
mod event;
mod input;
mod net;
mod particle;
mod renderer;
mod screens;

use std::time::Instant;

use raylib::ffi::KeyboardKey;

use crate::event::AppScreen;
use crate::screens::in_game::{GuiContext, InGameIds, InGameScene};
use crate::net::client::GameNetClient;
use crate::renderer::hud;
use crate::renderer::shader_manager::ShaderManager;
use crate::renderer::texture_manager::TextureManager;
use crate::renderer::types::RenderContext;
use crate::renderer::Renderer;
use crate::screens::main_menu::MenuAction;

use ui::context::UiContext;
use ui::draw::DrawCommandBuffer;

fn main() {
    let client_id = rand::random::<u64>();
    let mut renderer = Renderer::new(1280, 720);
    let mut client: Option<GameNetClient> = None;
    let mut screen = AppScreen::MainMenu;
    let mut draw_buffer = DrawCommandBuffer::new(4046);
    let mut ui_ctx = UiContext::new(renderer.screen_w as f32, renderer.screen_h as f32);
    let mut shader_manager = ShaderManager::new();
    let texture_manager = TextureManager::new();

    let mut last_frame = Instant::now();
    let mut is_solo: bool = false;

    // Scène de jeu
    let mut ingame_scene = InGameScene::default();

    // Shaders & UI HUD
    let sh_pr_bar = include_str!("../../shader/progress_bar.frag");
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

    while !renderer.rl.window_should_close() {
        let frame_delta = last_frame.elapsed();
        last_frame = Instant::now();
        let dt = renderer.rl.get_frame_time();

        if let Some(ref mut c) = client {
            c.update(frame_delta);

            while let Some(msg) = c.recv_lobby_message() {
                screens::lobby::handle_lobby_message(msg, &mut screen, &mut is_solo);
            }
        }

        match &mut screen {
            AppScreen::MainMenu => {
                let action =
                    screens::main_menu::handle_input(&renderer.rl, &mut client, client_id);

                match action {
                    MenuAction::Solo => {
                        is_solo = true;
                        println!("SOLO");
                    }
                    MenuAction::Multi => {
                        is_solo = false;
                        println!("MULTI");
                    }
                    MenuAction::None => {}
                }

                // Rendu Main Menu
                let mut d = renderer.rl.begin_drawing(&renderer.thread);
                match &client {
                    None => screens::main_menu::render(&mut d, &renderer.screen_scale),
                    Some(_) => {
                        screens::main_menu::render_connecting(&mut d, &renderer.screen_scale)
                    }
                }
            }

            AppScreen::Lobby(state) => {
                if let Some(ref mut c) = client {
                    screens::lobby::handle_input(&renderer.rl, state, c);
                    c.flush();
                }

                // Rendu Lobby
                let mut d = renderer.rl.begin_drawing(&renderer.thread);
                screens::lobby::render(&mut d, state, &renderer.screen_scale);
            }

            AppScreen::InGame(client_state) => {
                let client = client.as_mut().expect("InGame sans client réseau");

                let mut gui_ctx = GuiContext {
                    ui_ctx: &mut ui_ctx,
                    shader_manager: &mut shader_manager,
                    ids: &ingame_ids,
                };

                // Mise à jour logique de la partie
                ingame_scene.update(client, &mut renderer, client_state, &mut gui_ctx, dt);

                // Rendu de la frame
                ingame_scene.render(
                    &mut renderer,
                    client_state,
                    &mut RenderContext {
                        buffer: &mut draw_buffer,
                        texture_manager: &texture_manager,
                        shader_manager: &mut shader_manager,
                        ui_ctx: &mut ui_ctx,
                    },
                );
            }
        }

        if renderer.rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            std::process::exit(0);
        }
    }
}
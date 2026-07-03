mod camera;
mod config;
mod event;
mod input;
mod net;
mod particle;
mod renderer;
mod screens;

use net::client::GameNetClient;
use raylib::ffi::{KeyboardKey, MouseButton};
use raylib::prelude::*;
use renderer::Renderer;
use shared::protocol::StateSnapshot;
use shared::protocol::{ShopAction, ShopActionKind};
use std::time::{Duration, Instant};
use ui::context::UiContext;
use ui::draw::DrawCommandBuffer;
use ui::node::Anchor;
use ui::node::LayoutProps;
use ui::node::{VisualKind, VisualProps};
use ui::shader::ShaderRegistry;
use ui::texture::TextureRegistry;
use ui::tween::{easing, TweenProperty, Tween};

use crate::event::AppScreen;
use crate::particle::ParticleSystem;
use crate::screens::main_menu::MenuAction;

const TICK_DURATION: Duration = Duration::from_millis(50);

fn main() {
    let client_id = rand::random::<u64>();
    let mut renderer = Renderer::new(1280, 720);
    let mut client: Option<GameNetClient> = None;
    let mut screen = AppScreen::MainMenu;
    let mut draw_buffer = DrawCommandBuffer::new(4046);
    let tex_registry = TextureRegistry::new();
    let mut shader_registry = ShaderRegistry::new();
    let mut ui_ctx = UiContext::new(renderer.screen_w as f32, renderer.screen_h as f32);

    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut tick_id = 0u64;
    let mut prev_snapshot: Option<StateSnapshot> = None;
    let mut last_snapshot: Option<StateSnapshot> = None;
    let mut last_snap_time: Instant = Instant::now();
    let mut particle_system = ParticleSystem::new();
    let mut is_solo: bool = false;

    let shader_src = include_str!("../../shader/test.frag");
    let raw_shader = renderer
        .rl
        .load_shader_from_memory(&renderer.thread, None, Some(shader_src));
    let shader_id = shader_registry.register(raw_shader);

    let node_id = ui_ctx.add_node(
        ui_ctx.root,
        LayoutProps::new(
            Anchor::TopRight,
            Vector2::new(20.0, 20.0),
            Vector2::new(180.0, 40.0),
        ),
        VisualProps {
            kind: VisualKind::Shader { id: shader_id },
            color: Color::RED,
            visible: true,
            opacity: 1.0,
        },
    );
    ui_ctx.tween.add(Tween {
        target: node_id,
        property: TweenProperty::Opacity { from: 0.0, to: 1.0 },
        duration: 2.0,
        elapsed: 0.0,
        easing: easing::ease_in_out_quad,
        done: false,
    });

    // tick réseau 20 Hz
    while !renderer.rl.window_should_close() {
        let frame_delta = last_frame.elapsed();
        last_frame = Instant::now();

        if let Some(ref mut c) = client {
            c.update(frame_delta);

            // Lobby messages — actifs dans tous les écrans
            while let Some(msg) = c.recv_lobby_message() {
                screens::lobby::handle_lobby_message(msg, &mut screen, &mut is_solo);
            }
        }

        match &mut screen {
            AppScreen::MainMenu => {
                let action = screens::main_menu::handle_input(&renderer.rl, &mut client, client_id);

                match action {
                    MenuAction::Solo => {
                        is_solo = true;
                        println!("SOLO")
                    }
                    MenuAction::Multi => {
                        is_solo = false;
                        println!("MULTI")
                    }
                    MenuAction::None => {}
                }

                // recv ici aussi pour capter le Sessionjoined
                if let Some(ref mut c) = client {
                    c.update(frame_delta);

                    // Lobby messages — actifs dans tous les écrans
                    while let Some(msg) = c.recv_lobby_message() {
                        screens::lobby::handle_lobby_message(msg, &mut screen, &mut is_solo);
                        c.flush();
                    }
                }

                {
                    let mut d = renderer.rl.begin_drawing(&renderer.thread);
                    match &client {
                        None => screens::main_menu::render(&mut d, &renderer.screen_scale),
                        Some(_) => {
                            screens::main_menu::render_connecting(&mut d, &renderer.screen_scale)
                        }
                    }
                }
            }
            AppScreen::Lobby(state) => {
                if let Some(ref mut c) = client {
                    screens::lobby::handle_input(&renderer.rl, state, c);
                    c.flush();
                }
                let mut d = renderer.rl.begin_drawing(&renderer.thread);
                screens::lobby::render(&mut d, state, &renderer.screen_scale);
            }
            AppScreen::InGame(client_state) => {
                let client = client.as_mut().expect("InGame sans client réseau");
                if renderer.rl.is_key_pressed(KeyboardKey::KEY_F2) {
                    client_state.debug.cycle();

                    // TODO: A ajouter quand il y aura un réticule
                    // if client_state.debug.mode == DebugMode::Interactive {
                    //     renderer.rl.show_cursor();
                    // } else {
                    //     renderer.rl.hide_cursor();
                    // }
                }

                // réception à chaque frame, pas seulement au tick
                while let Some(snap) = client.recv_snapshot() {
                    prev_snapshot = last_snapshot.take();
                    last_snapshot = Some(snap);
                    last_snap_time = Instant::now();
                }

                client_state.debug.cleared = false;
                while let Some(event) = client.recv_event() {
                    client_state.handle_event(event);
                }

                input::handle_shop_input(&renderer.rl, client, client_state);

                if client_state.phase.can_show_shop()
                    && renderer
                        .rl
                        .is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
                {
                    let mouse = renderer.rl.get_mouse_position();

                    let card_y = renderer.screen_scale.y(config::SHOP_CARD_Y);
                    let card_w = renderer.screen_scale.w(config::SHOP_CARD_W);
                    let card_h = renderer.screen_scale.h(config::SHOP_CARD_H);
                    let clicked = config::SHOP_SLOTS_X.iter().enumerate().find(|&(_, &x)| {
                        let card_x = renderer.screen_scale.x(x);

                        mouse.x as i32 >= card_x
                            && mouse.x as i32 <= card_x + card_w
                            && mouse.y as i32 >= card_y
                            && mouse.y as i32 <= card_y + card_h
                    });

                    if let Some((slot, _)) = clicked {
                        client.send_shop_action(&ShopAction {
                            kind: ShopActionKind::Buy,
                            slot: slot as u8,
                        });
                    }
                }

                // tick réseau 20 Hz — envoi uniquement
                if last_tick.elapsed() >= TICK_DURATION {
                    last_tick = Instant::now();

                    if client.is_connected() {
                        let packet = input::read_input(
                            &renderer.rl,
                            tick_id,
                            renderer.screen_w,
                            renderer.screen_h,
                        );
                        client.send_input(&packet);
                    }

                    client.flush();
                    tick_id += 1;
                }

                // caméra
                if let Some(curr) = &last_snapshot {
                    let t = (last_snap_time.elapsed().as_secs_f32() / TICK_DURATION.as_secs_f32())
                        .clamp(0.0, 1.0);
                    camera::update(&mut renderer.cam, prev_snapshot.as_ref(), curr, t);
                }

                ui_ctx.update(frame_delta.as_secs_f32());
                // rendu 60 Hz
                renderer.render_frame(
                    prev_snapshot.as_ref(),
                    last_snapshot.as_ref(),
                    last_snap_time,
                    client_state,
                    &mut particle_system,
                    &mut draw_buffer,
                    &tex_registry,
                    &mut shader_registry,
                    &mut ui_ctx,
                );
            }
        }

        //client_state.debug.collider.clear();
        if renderer.rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            std::process::exit(0);
        }
    }
}

mod camera;
mod config;
mod event;
mod input;
mod net;
mod particle;
mod renderer;
mod screens;

use crate::input::ShopInputAction;
use crate::particle::Particle;
use crate::renderer::hud;
use crate::renderer::shader_manager::ShaderManager;
use crate::renderer::texture_manager::TextureManager;
use crate::renderer::types::{FrameState, RenderContext};
use net::client::GameNetClient;
use raylib::ffi::{KeyboardKey, MouseButton};
use raylib::prelude::*;
use renderer::Renderer;
use shared::protocol::EntityKind;
use shared::protocol::StateSnapshot;
use shared::protocol::{ShopAction, ShopActionKind};
use std::time::{Duration, Instant};
use ui::context::UiContext;
use ui::draw::DrawCommandBuffer;
use ui::event::UIEvent;
use ui::node::UiUnit;
use ui::node::UiVec2;

use crate::event::{AppScreen, handle_shop_ui_event};
use crate::particle::ParticleSystem;
use crate::screens::main_menu::MenuAction;

const TICK_DURATION: Duration = Duration::from_millis(50);

fn main() {
    let client_id = rand::random::<u64>();
    let mut renderer = Renderer::new(1280, 720);
    let mut client: Option<GameNetClient> = None;
    let mut screen = AppScreen::MainMenu;
    let mut draw_buffer = DrawCommandBuffer::new(4046);
    let mut ui_ctx = UiContext::new(renderer.screen_w as f32, renderer.screen_h as f32);
    let mut shader_manager = ShaderManager::new();
    let mut texture_manager = TextureManager::new();

    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut tick_id = 0u64;
    let mut prev_snapshot: Option<StateSnapshot> = None;
    let mut last_snapshot: Option<StateSnapshot> = None;
    let mut last_snap_time: Instant = Instant::now();
    let mut particle_system = ParticleSystem::new();
    let mut is_solo: bool = false;


    let sh_pr_bar = include_str!("../../shader/progress_bar.frag");
    let raw_sh = renderer
        .rl
        .load_shader_from_memory(&renderer.thread, None, Some(sh_pr_bar));
    let sh_pr_id = shader_manager.register(raw_sh);

    let hud_node_id = hud::init_hud(&mut ui_ctx, sh_pr_id);
    let shop_ids = hud::init_shop(&mut ui_ctx);

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
                let dt = renderer.rl.get_frame_time();
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

                while let Some(snap) = client.recv_snapshot() {
                    prev_snapshot = last_snapshot.take();
                    last_snapshot = Some(snap);
                    last_snap_time = Instant::now();
                }

                if let Some(snap) = &last_snapshot {
                    // MAJ HUD
                    // TODO: A terme avoir une fonction libre qui update les elements du HUD (une par element ou une global)
                    if let Some(info) = &snap.player_info {
                        let ratio = info.health / info.max_health;

                        ui_ctx.send_event(UIEvent::SetSize {
                            target: hud_node_id.hp_fill_id,
                            size: UiVec2::new(
                                UiUnit::ParentPercent(ratio),
                                UiUnit::ParentPercent(1.0),
                            ),
                        });
                        ui_ctx.send_event(UIEvent::SetText {
                            target: hud_node_id.hp_text_id,
                            content: format!("{} / {}", info.health, info.max_health),
                        });
                        ui_ctx.send_event(UIEvent::SetText {
                            target: hud_node_id.gold_label_id,
                            content: format!("Or {}", info.gold),
                        });

                        if let Some(shader) = shader_manager.get_mut(sh_pr_id) {
                            let loc = shader.get_shader_location("u_ratio");
                            shader.set_shader_value(loc, ratio);
                        }
                    }

                    {
                        let wave_info = &snap.wave_info;
                        ui_ctx.send_event(UIEvent::SetText {
                            target: hud_node_id.wave_label_id,
                            content: format!(
                                "Vague {} | Enemis {}",
                                wave_info.wave_number, wave_info.enemy_remaining
                            ),
                        });
                    }

                    // Spawn Particle
                    for entity in &snap.entities {
                        let prev_entity = prev_snapshot.as_ref().and_then(|p| {
                            p.entities.iter().find(|e| e.entity_id == entity.entity_id)
                        });

                        let t = (last_snap_time.elapsed().as_secs_f32()
                            / TICK_DURATION.as_secs_f32())
                        .clamp(0.0, 1.0);

                        let (x, y) = match prev_entity {
                            Some(prev) => (
                                lerp(prev.position[0], entity.position[0], t),
                                lerp(prev.position[1], entity.position[1], t),
                            ),
                            None => (entity.position[0], entity.position[1]),
                        };
                        if matches!(entity.entity_kind, EntityKind::Player) {
                            if let Some(prev) = prev_entity {
                                let dx = entity.position[0] - prev.position[0];
                                let dy = entity.position[1] - prev.position[1];
                                if dx.abs() > 0.05 || dy.abs() > 0.05 {
                                    let lifetime = rand::random_range(0.18..0.32f32);
                                    particle_system.spawn(Particle {
                                        pos: Vector2 {
                                            x: x + rand::random_range(-20.0..20.0),
                                            y: y + 20.0,
                                        },
                                        velocity: Vector2 {
                                            x: (-dx * 4.0) + rand::random_range(-20.0..20.0),
                                            y: rand::random_range(-50.0..-20.0),
                                        },
                                        friction: 4.5,
                                        lifetime,
                                        lt_max: lifetime,
                                        scale: 0.1,
                                        growth: 6.5,
                                        color: Color::LIGHTGRAY,
                                    });
                                }
                            }
                        }
                    }
                }

                let mouse_pos = renderer.rl.get_mouse_position();
                let pressed = renderer
                    .rl
                    .is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
                let released = renderer
                    .rl
                    .is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT);

                let events = ui_ctx.process_input(mouse_pos, pressed, released);
                for event in events {
                    match event {
                        _ => {}
                    }
                }

                client_state.debug.cleared = false;
                while let Some(event) = client.recv_event() {
                    handle_shop_ui_event(&event, &mut ui_ctx, &shop_ids);
                    client_state.handle_event(event);
                }

                match input::handle_shop_input(&renderer.rl, client, client_state) {
                    ShopInputAction::Close => {
                        ui_ctx.send_event(UIEvent::SetVisible {
                            target: shop_ids.root,
                            visible: false,
                        });
                        client_state.close_shop();
                    }
                    ShopInputAction::Open => {}
                    ShopInputAction::None => {}
                }

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

                // Maj logique
                {
                    client_state.update_timers(dt);
                    particle_system.update(dt);
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
                    FrameState {
                        current: last_snapshot.as_ref(),
                        prev: prev_snapshot.as_ref(),
                        last_snap_time,
                    },
                    client_state,
                    &particle_system,
                    &mut RenderContext {
                        buffer: &mut draw_buffer,
                        texture_manager: &texture_manager,
                        shader_manager: &mut shader_manager,
                        ui_ctx: &mut ui_ctx,
                    },
                );
            }
        }

        //client_state.debug.collider.clear();
        if renderer.rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            std::process::exit(0);
        }
    }
}

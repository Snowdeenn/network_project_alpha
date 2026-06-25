mod camera;
mod config;
mod event;
mod input;
mod net;
mod particle;
mod renderer;

use event::ClientState;
use net::client::GameNetClient;
use raylib::ffi::{KeyboardKey, MouseButton};
use renderer::Renderer;
use shared::protocol::StateSnapshot;
use shared::protocol::{ShopAction, ShopActionKind};
use std::time::{Duration, Instant};

use crate::particle::ParticleSystem;

const TICK_DURATION: Duration = Duration::from_millis(50);

fn main() {
    let client_id = rand::random::<u64>();
    let mut renderer = Renderer::new(1280, 720);
    let mut client = GameNetClient::new(client_id);
    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut tick_id = 0u64;
    let mut prev_snapshot: Option<StateSnapshot> = None;
    let mut last_snapshot: Option<StateSnapshot> = None;
    let mut last_snap_time: Instant = Instant::now();
    let mut client_state = ClientState::new();
    let mut particle_system = ParticleSystem::new();

    // tick réseau 20 Hz
    while !renderer.rl.window_should_close() {
        let frame_delta = last_frame.elapsed();
        last_frame = Instant::now();
        client.update(frame_delta);

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

        input::handle_shop_input(&renderer.rl, &mut client, &mut client_state);

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
                let packet =
                    input::read_input(&renderer.rl, tick_id, renderer.screen_w, renderer.screen_h);
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

        // rendu 60 Hz
        renderer.render_frame(
            prev_snapshot.as_ref(),
            last_snapshot.as_ref(),
            last_snap_time,
            &mut client_state,
            &mut particle_system,
        );

        //client_state.debug.collider.clear();
        if renderer.rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            std::process::exit(0);
        }
    }
}

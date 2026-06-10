mod camera;
mod event;
mod input;
mod net;
mod renderer;
mod config;

use event::ClientState;
use net::client::GameNetClient;
use raylib::ffi::MouseButton;
use renderer::Renderer;
use shared::protocol::{StateSnapshot};
use std::time::{Duration, Instant};
use shared::protocol::{ShopAction, ShopActionKind};


const TICK_DURATION: Duration = Duration::from_millis(50);
const SCREEN_W: i32 = 1366;
const SCREEN_H: i32 = 768;

fn main() {
    let mut renderer = Renderer::new(SCREEN_W, SCREEN_H);
    let mut client = GameNetClient::new(1);
    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut tick_id = 0u64;
    let mut prev_snapshot: Option<StateSnapshot> = None;
    let mut last_snapshot: Option<StateSnapshot> = None;
    let mut last_snap_time: Instant = Instant::now();
    let mut client_state = ClientState::new();

    // tick réseau 20 Hz
    while !renderer.rl.window_should_close() {
        let frame_delta = last_frame.elapsed();
        last_frame = Instant::now();
        client.update(frame_delta);

        // réception à chaque frame, pas seulement au tick
        while let Some(snap) = client.recv_snapshot() {
            prev_snapshot = last_snapshot.take();
            last_snapshot = Some(snap);
            last_snap_time = Instant::now();
        }

        while let Some(event) = client.recv_event() {
            client_state.handle_event(event);
        }

        input::handle_shop_input(&renderer.rl, &mut client, &mut client_state);

        if client_state.show_shop && renderer.rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let mouse = renderer.rl.get_mouse_position();
            let slots_x = [335, 785, 1235];

            let clicked = slots_x.iter().enumerate().find(|&(_, &x)| {
                mouse.x >= x as f32
                    && mouse.x <= (x + 350) as f32
                    && mouse.y >= 290.0
                    && mouse.y <= 790.0
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
                let packet = input::read_input(&renderer.rl, tick_id, SCREEN_W, SCREEN_H);
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
        );
    }
}

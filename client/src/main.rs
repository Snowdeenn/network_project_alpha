mod camera;
mod input;
mod net;
mod renderer;

use net::client::GameNetClient;
use renderer::Renderer;
use shared::protocol::StateSnapshot;
use std::time::{Duration, Instant};

const TICK_DURATION: Duration = Duration::from_millis(50);
const SCREEN_W: i32 = 1920;
const SCREEN_H: i32 = 1080;

fn main() {
    let mut renderer      = Renderer::new(SCREEN_W, SCREEN_H);
    let mut client        = GameNetClient::new(1);
    let mut last_tick     = Instant::now();
    let mut tick_id       = 0u64;
    let mut last_snapshot: Option<StateSnapshot> = None;

    while !renderer.rl.window_should_close() {
        // tick réseau 20 Hz
        if last_tick.elapsed() >= TICK_DURATION {
            last_tick = Instant::now();
            client.update(last_tick.elapsed());

            if client.is_connected() {
                let packet = input::read_input(&renderer.rl, tick_id, SCREEN_W, SCREEN_H);
                client.send_input(&packet);

                while let Some(snap) = client.recv_snapshot() {
                    last_snapshot = Some(snap);
                }
            }

            client.flush();
            tick_id += 1;
        }

        // caméra
        if let Some(snap) = &last_snapshot {
            camera::update(&mut renderer.cam, snap);
        }

        // rendu 60 Hz
        renderer.render_frame(last_snapshot.as_ref());
    }
}
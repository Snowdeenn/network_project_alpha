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
    let mut renderer = Renderer::new(SCREEN_W, SCREEN_H);
    let mut client = GameNetClient::new(1);
    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut tick_id = 0u64;
    let mut prev_snapshot: Option<StateSnapshot> = None;
    let mut last_snapshot: Option<StateSnapshot> = None;
    let mut last_snap_time: Instant = Instant::now();

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
        );
    }
}

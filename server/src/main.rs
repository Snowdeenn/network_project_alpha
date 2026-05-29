mod net;
use net::server::*;
use renet::ServerEvent;
use std::time::{Duration, Instant};

const TICK_RATE: u64 = 20;
const TICK_DURATION: Duration = Duration::from_millis(1000 / TICK_RATE);

fn main() {
    let mut net = GameNetServer::new();
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let delta = now - last;
        last = now;

        net.update(delta);

        for event in net.drain_events() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    println!("Client connecté : {}", client_id);
                }
                ServerEvent::ClientDisconnected { client_id, .. } => { /* cleanup */ }
            }
        }

        // 3. inputs → simulation (à brancher sur l'ECS plus tard)
        let inputs = net.drain_inputs();
        for (client_id, input) in inputs {
            println!(
                "[client {}] input tick {} — move {:?} dash {}",
                client_id, input.tick_id, input.move_dir, input.dash
            );
        }
        let shops = net.drain_shop_actions();

        // 4. broadcast snapshot
        // net.broadcast_snapshot(&snapshot);

        // 5. flush
        net.flush();

        // 6. sleep
        let elapsed = last.elapsed();
        if elapsed < TICK_DURATION {
            std::thread::sleep(TICK_DURATION - elapsed);
        }
    }
}

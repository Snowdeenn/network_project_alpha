mod net;
use std::time::{Duration, Instant};
use net::client::GameNetClient;
use shared::protocol::InputPacket;

const TICK_DURATION: Duration = Duration::from_millis(50);

fn main() {
    let mut client = GameNetClient::new(1); // client_id arbitraire pour le test
    let mut last   = Instant::now();
    let mut tick   = 0u64;

    println!("Connexion à 127.0.0.1:7777...");

    loop {
        let delta = last.elapsed();
        last = Instant::now();

        client.update(delta);

        if client.is_connected() {
            // envoie un input factice à chaque tick
            client.send_input(&InputPacket {
                tick_id:  tick,
                move_dir: [1.0, 0.0],
                dash:     tick % 40 == 0, // dash toutes les 2s
                spell:    None,
                aim_dir:  [1.0, 0.0],
            });

            // affiche ce qu'on reçoit
            if let Some(snapshot) = client.recv_snapshot() {
                println!("[tick {}] snapshot reçu — {} entités", snapshot.tick_id, snapshot.entities.len());
            }
            while let Some(event) = client.recv_event() {
                println!("[tick {}] event reçu — {:?}", tick, event);
            }
        }

        client.flush();

        tick += 1;
        let elapsed = last.elapsed();
        if elapsed < TICK_DURATION {
            std::thread::sleep(TICK_DURATION - elapsed);
        }
    }
}
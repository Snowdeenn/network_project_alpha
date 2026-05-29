mod net;
mod simulation;
mod snapshot;

use legion::*;
use net::server::GameNetServer;
use renet::ServerEvent;
use simulation::{
    components::*,
    eco::*,
    event::*,
    input::{InputQueue, InputState},
    systems::*,
    wave::*,
    helper::clear_resource_queues,
};
use snapshot::build_snapshot;
use std::time::{Duration, Instant};

const TICK_DURATION: Duration = Duration::from_millis(50); // 20 Hz

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Playing,
    Shop,
}

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut net = GameNetServer::new();
    let mut world = World::default();
    let mut resources = Resources::default();

    // --- resources ---
    {
        resources.insert(Duration::new(0, 0));
        resources.insert(InputQueue(vec![]));
        resources.insert(InputState::default());
        resources.insert(DamageQueue(vec![]));
        resources.insert(EnemyDiedQueue(vec![]));
        resources.insert(CoinSpawnQueue(vec![]));
        resources.insert(PickupQueue(vec![]));
        resources.insert(Gold(0));
        resources.insert(GameState::Playing);
        resources.insert(simulation::helper::PlayerPos { x: 0.0, y: 0.0 });
    }

    // --- wave config ---
    {
        let wave_json = std::fs::read_to_string("assets/wave.json")?;
        let wave_configs: Vec<WaveConfig> = serde_json::from_str(&wave_json)?;

        resources.insert(WaveManager {
            current_wave: 0,
            enemies_remaining: wave_configs[0].enemy_count,
            enemies_to_spawn: wave_configs[0].enemy_count,
            spawn_timer: Duration::from_millis(wave_configs[0].spawn_interval),
            wave_state: WaveState::InProgress,
        });
        resources.insert(WaveConfigs(wave_configs));
    }

    // --- Enemy Pool ---
    {
        let mut pool = EnemyPool { pool: vec![] };
        for _ in 0..100 {
            let e = world.push((
                EntityId(next_id()),
                IA,
                Position { x: 0.0, y: 0.0 },
                Velocity { dx: 0.0, dy: 0.0 },
                Collider { w: 40.0, h: 40.0 },
                Health {
                    hp: 100,
                    state: HealthState::Alive,
                },
                Active(false),
            ));
            pool.pool.push(e);
        }
        resources.insert(pool)
    };

    {
        let mut coin_pool = CoinPool { coins: vec![] };
        for _ in 0..50 {
            let e = world.push((
                EntityId(next_id()),
                Coin,
                Position { x: 0.0, y: 0.0 },
                Collider { w: 20.0, h: 20.0 },
                Active(false),
                CoinValue(rand::random::<u32>() % 10 + 1),
            ));
            coin_pool.coins.push(e);
        }
        resources.insert(coin_pool);
    }

    let mut schedule = Schedule::builder()
        .add_system(friction_system())
        .add_system(update_velocity_system())
        .add_system(dash_system())
        .add_system(update_position_system())
        .add_system(ia_seek_system())
        .add_system(collide_system())
        .add_system(collide_arena_system())
        .add_system(apply_damage_system())
        .add_system(health_system())
        .add_system(coin_push_to_queue_system())
        .add_system(coin_spawn_system())
        .add_system(coin_pickup_system())
        .add_system(apply_pickup_system())
        .add_system(wave_update_system())
        .build();

    let mut tick_id = 0u64;
    let mut last = Instant::now();

    println!("Serveur démarré sur 127.0.0.1:7777");

    loop {
        let delta = last.elapsed();
        last = Instant::now();

        net.update(delta);

        for event in net.drain_events() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    println!("Client connecté : {}", client_id);

                    world.push((
                        EntityId(client_id),
                        Player,
                        Position { x: 960.0, y: 540.0 },
                        Velocity { dx: 0.0, dy: 0.0 },
                        Dash(DashState::Idle),
                        Collider { w: 40.0, h: 40.0 },
                        Health {
                            hp: 100,
                            state: HealthState::Alive,
                        },
                        Active(true),
                    ));
                }
                ServerEvent::ClientDisconnected { client_id: _, .. } => {
                    println!("Client déconnecté");
                    // TODO : supprimer l'entité joueur
                }
            }
        }

        let inputs = net.drain_inputs();
        if let Some((_, packet)) = inputs.first() {
            if let Some(mut state) = resources.get_mut::<InputState>() {
                state.move_dir = packet.move_dir;
                state.aim_dir = packet.aim_dir;
                state.dash = packet.dash;
                state.spell = packet.spell;
            }
        }

        if let Some(mut res_dt) = resources.get_mut::<Duration>() {
            *res_dt = TICK_DURATION; // timestep fixe côté serveur
        }
        schedule.execute(&mut world, &mut resources);

        {
          let snapshot = build_snapshot(&world, &resources, tick_id);
            net.broadcast_snapshot(&snapshot);  
        }
        

        net.flush();
        clear_resource_queues(&mut resources);

        tick_id += 1;
        {
           let elapsed = last.elapsed();
            if elapsed < TICK_DURATION {
                std::thread::sleep(TICK_DURATION - elapsed);
            } 
        }
        
    }
}

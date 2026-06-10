mod net;
mod simulation;
mod snapshot;

use crate::simulation::helper::{PlayerHp, PlayerPos};
use legion::{Entity, EntityStore, IntoQuery, Resources, Schedule, component, world::World};
use net::server::GameNetServer;
use renet::ServerEvent;
use shared::protocol::{
    GameEvent, GameEventKind, InputPacket, ShopAction, ShopActionKind, ShopItem,
};
use simulation::shop::PlayerShops;
use simulation::{
    components::*, eco::*, event::*, helper::clear_resource_queues, input::InputQueue, systems::*,
    wave::*,
};
use snapshot::build_snapshot;
use std::collections::HashMap;
use std::time::{Duration, Instant};
const TICK_DURATION: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Playing,
    Shop,
}

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1000);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut net = GameNetServer::new();
    let mut world = World::default();
    let mut resources = Resources::default();
    let mut players_entities: HashMap<u64, Entity> = HashMap::new();

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
        resources.insert(PlayerPos { x: 0.0, y: 0.0 });
        resources.insert(PlayerHp {
            hp: 100.0,
            max_hp: 100.0,
        });
        resources.insert(GameEventQueue(vec![]));
        resources.insert(PlayerShops::new());
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

    // --- Coin Pool ---
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

    // ---- Items Pool ----
    {
        let items_json = std::fs::read_to_string("assets/items.json")?;
        let items: Vec<Option<ShopItem>> = serde_json::from_str(&items_json)?;
        resources.insert(ItemPool { items });
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

                    let entity = world.push((
                        EntityId(next_id()),
                        Player,
                        InputState::default(),
                        Position { x: 960.0, y: 540.0 },
                        Velocity { dx: 0.0, dy: 0.0 },
                        Dash(DashState::Idle),
                        Collider { w: 40.0, h: 40.0 },
                        Health {
                            hp: 100,
                            state: HealthState::Alive,
                        },
                    ));

                    let mut entry = world.entry(entity).unwrap();
                    entry.add_component(Active(true));

                    players_entities.insert(client_id, entity);
                }
                ServerEvent::ClientDisconnected { client_id, .. } => {
                    println!("Client déconnecté {}", client_id);
                    if let Option::Some(entity) = players_entities.remove(&client_id) {
                        world.remove(entity);
                    }
                }
            }
        }

        // Traite les inputs reçus du client et les stocke dans les ressources globales
        {
            for (client_id, packet) in net.drain_inputs() {
                if let Some(&entity) = players_entities.get(&client_id) {
                    apply_input(&mut world, entity, &packet);
                }
            }
        }

        {
            for (client_id, shop_action) in net.drain_shop_actions() {
                handle_shop_action(client_id, &mut net, shop_action, &mut resources);
            }
        }

        // Met à jour le tick actuel dans les ressources pour que les systèmes puissent y accéder
        if let Some(mut res_dt) = resources.get_mut::<Duration>() {
            *res_dt = TICK_DURATION; // timestep fixe côté serveur
        }

        // Met à jour la position et la santé du joueur dans les ressources pour que les systèmes IA puissent y accéder
        {
            let mut player_query = <(&Position, &Health)>::query()
                .filter(component::<Player>() & component::<Active>());
            if let Some((pos, health)) = player_query.iter(&world).next() {
                if let Some(mut player_pos) = resources.get_mut::<PlayerPos>() {
                    player_pos.x = pos.x;
                    player_pos.y = pos.y;
                }
                if let Some(mut player_hp) = resources.get_mut::<PlayerHp>() {
                    player_hp.hp = health.hp.into();
                }
            }
        }

        schedule.execute(&mut world, &mut resources);

        {
            let snapshot = build_snapshot(&world, &resources, tick_id);
            net.broadcast_snapshot(&snapshot);
        }

        {
            let mut game_events = resources.get_mut::<GameEventQueue>().unwrap();
            for event in game_events.0.drain(..) {
                net.broadcast_event(&event);
            }
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

fn apply_input(world: &mut World, entity: Entity, packet: &InputPacket) {
    if let Ok(mut entry) = world.entry_mut(entity) {
        if let Ok(state) = entry.get_component_mut::<InputState>() {
            state.move_dir = packet.move_dir;
            state.aim_dir = packet.aim_dir;
            state.dash = packet.dash;
            state.spell = packet.spell;
        }
    }
}

fn handle_shop_action(
    client: u64,
    server: &mut GameNetServer,
    action: ShopAction,
    res: &mut Resources,
) {
    let mut player_shop = res.get_mut::<PlayerShops>().unwrap();

    match action.kind {
        ShopActionKind::Open => {
            println!("Client {} à ouvert le shop", client);

            let item_pool = res.get::<ItemPool>().unwrap();
            let shop_inventory = player_shop.generate(client, &item_pool.items);
            server.send_event(
                client,
                &GameEvent {
                    kind: GameEventKind::ShopOpened {
                        inventory: shop_inventory,
                    },
                },
            );
        }
        ShopActionKind::Buy => {
            println!("Client {} à acheté un item du shop", client);
            let item = player_shop.buy(client, action.slot as usize, res);
            match item {
                Some(item) => {
                    server.send_event(client, &GameEvent { kind: GameEventKind::ItemBought { item }});
                },
                None => {
                    println!("Client {} n'a pas pu acheter l'item du slot {}", client, action.slot);
                    server.send_event(client, &GameEvent { kind: GameEventKind::PurchaseFailed });
                }
            }
        }
        ShopActionKind::Close => {
            println!("Client {} à fermer le shop", client);
        }
    }
}

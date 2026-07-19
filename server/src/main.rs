mod config;
mod lobby;
mod net;
mod player_registry;
pub mod pool;
pub mod queue;
mod session;
mod simulation;
mod snapshot;

use crate::config::*;
use crate::player_registry::PlayerRegistry;
use crate::pool::{GamePools, PoolManager};
use crate::queue::Queue;
use crate::session::*;
use crate::simulation::systems::{
    attack::*, coin::*, debug::*, health::*, ia::*, physics::*, state::dash_system, wave::*,
};

use legion::{Entity, EntityStore, Resources, Schedule, world::World};
use net::server::GameNetServer;
use renet::ServerEvent;
use shared::buffer::BufferManager;
use shared::config::{ClassConfig, ClassRegistery, GameConfig, PlayerClass};
use shared::protocol::*;
use simulation::shop::PlayerShops;
use simulation::{components::*, eco::*, event::*, helper::clear_resource_queues, wave::*};
use snapshot::{build_entities, build_player_info, build_wave_info};
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

    // --- resources ---
    {
        resources.insert(Duration::new(0, 0));
        resources.insert(InputState::default());
        resources.insert(Queue::<DamageEvent> { data: vec![] });
        resources.insert(Queue::<InputPacket> { data: vec![] });
        resources.insert(Queue::<EnemyDied> { data: vec![] });
        resources.insert(Queue::<CoinEvent> { data: vec![] });
        resources.insert(Queue::<(Entity, Entity)> { data: vec![] });
        resources.insert(Queue::<GameEvent> { data: vec![] });
        resources.insert(GameState::Playing);
        resources.insert(PlayerShops::new());
        resources.insert(PlayerGold::new());
        resources.insert(PlayerRegistry::with_capacity(16));
        resources.insert(BufferManager::with_capacity(24));
    }

    // --- wave config ---
    {
        let wave_json = std::fs::read_to_string("assets/config/wave.json")
            .expect("assets/config/wave.json introuvable");
        let wave_configs: Vec<WaveConfig> =
            serde_json::from_str(&wave_json).expect("impossible de parser wave.json");

        resources.insert(WaveManager {
            current_wave: 0,
            enemies_remaining: wave_configs[0].enemy_count,
            enemies_to_spawn: wave_configs[0].enemy_count,
            spawn_timer: Duration::from_millis(wave_configs[0].spawn_interval_ms),
            wave_state: simulation::wave::WaveState::Waiting,
        });
        resources.insert(WaveConfigs(wave_configs));
    }

    // --- Enemy Config ---
    {
        let json = std::fs::read_to_string("assets/config/enemy_config.json")
            .expect("enemy_config.json introuvable");
        let enemy_config: HashMap<String, EnemyStatsConfig> =
            serde_json::from_str(&json).expect("impossible de parser enemy_config.json");

        resources.insert(EnemyConfigs(enemy_config));
    }

    {
        let pool_manager = PoolManager::new(GamePools::init(&mut world));
        resources.insert(pool_manager);
    }

    // ---- Items Pool ----
    {
        let items_json = std::fs::read_to_string("assets/config/items.json")?;
        let items: Vec<Option<ShopItem>> = serde_json::from_str(&items_json)?;
        resources.insert(ItemPool { items });
    }

    // --- Class Config ---
    {
        let warrior = std::fs::read_to_string("assets/classes/warrior.json")
            .expect("Le chemion ou les droit sur le fichier warrior.json ne sont pas bon");
        let assassin = std::fs::read_to_string("assets/classes/assassin.json")
            .expect("Le chemion ou les droit sur le fichier assassin.json ne sont pas bon");
        let mage = std::fs::read_to_string("assets/classes/mage.json")
            .expect("Le chemion ou les droit sur le fichier mage.json ne sont pas bon");
        let tank = std::fs::read_to_string("assets/classes/tank.json")
            .expect("Le chemion ou les droit sur le fichier tank.json ne sont pas bon");

        let p_w: ClassConfig =
            serde_json::from_str(&warrior).expect("Impossible de parser le ficher warrior.json");
        let p_a: ClassConfig =
            serde_json::from_str(&assassin).expect("Impossible de parser le ficher assassin.json");
        let p_m: ClassConfig =
            serde_json::from_str(&mage).expect("Impossible de parser le ficher mage.json");
        let p_t: ClassConfig =
            serde_json::from_str(&tank).expect("Impossible de parser le ficher tank.json");

        let mut config = HashMap::new();
        config.insert(PlayerClass::Warrior, p_w);
        config.insert(PlayerClass::Assassin, p_a);
        config.insert(PlayerClass::Mage, p_m);
        config.insert(PlayerClass::Tank, p_t);

        resources.insert(ClassRegistery { config });
    }

    // --- Game Config ---
    {
        let json = std::fs::read_to_string("assets/config/game_config.json")
            .expect("Le chemin ou les droits sur game_config.json ne sont pas bon");
        let game_config: GameConfig =
            serde_json::from_str(&json).expect("Impossible de parser le json game_config.json");

        resources.insert(SharedLives::new(game_config.shared_lives));
        resources.insert(game_config);
    }

    // --- Serveur Config ---
    {
        let json = std::fs::read_to_string("assets/config/server_config.json")
            .expect("Le chemin ou les droits sur server_config.json ne sont pas bon");
        let server_config: ServerConfig =
            serde_json::from_str(&json).expect("Impossible de parser le json server_config.json");

        resources.insert(server_config);
    }

    // --- Physics Config ---
    {
        let json = std::fs::read_to_string("assets/config/physics_config.json")
            .expect("Le chemin ou les droits sur physics_config.json ne sont pas bon");
        let physics_config: PhysicsConfig =
            serde_json::from_str(&json).expect("Impossible de parser le json physics_config.json");

        resources.insert(physics_config);
    }
    let mut schedule = Schedule::builder()
        .add_system(ia_targeting_system())
        .add_system(friction_system())
        .add_system(update_velocity_system())
        .add_system(melee_ia_movement_system())
        .add_system(ranged_ia_movement_system())
        .add_system(knockback_system())
        .add_system(dash_system())
        .add_system(update_position_system())
        .add_system(collide_system())
        .add_system(collide_arena_system())
        .add_system(projectile_life_time_system())
        .add_system(read_player_attack_intent_system())
        .add_system(ia_attack_system())
        .add_system(create_attack_box_system())
        .add_system(check_collide_attackbox_system())
        .add_system(kamikaze_suicide_system())
        .add_system(apply_damage_system())
        .add_system(health_system())
        .add_system(coin_push_to_queue_system())
        .add_system(coin_spawn_system())
        .add_system(coin_pickup_system())
        .add_system(apply_pickup_system())
        .add_system(wave_death_reaper_system())
        .add_system(wave_spawner_system())
        .add_system(wave_flow_manager_system())
        .add_system(send_collider_system())
        .add_system(debug_projectile_positions_system())
        .build();

    let mut tick_id = 0u64;
    let mut last = Instant::now();

    let mut session = {
        let server_cfg = resources.get::<ServerConfig>().unwrap();
        SessionState::new(&server_cfg)
    };
    println!("Code de session : {}", session.code);

    println!("Serveur démarré sur 127.0.0.1:7777");

    loop {
        let delta = last.elapsed();
        last = Instant::now();

        net.update(delta);

        for event in net.drain_events() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    println!("Client connected : {}", client_id);
                    let game_cfg = resources.get::<GameConfig>().unwrap();
                    match session.add_slot(client_id, &game_cfg) {
                        Some(slot_index) => {
                            resources
                                .get_mut::<PlayerRegistry>()
                                .unwrap()
                                .add(client_id);

                            net.send_lobby(
                                client_id,
                                &LobbyMessage::SessionJoined {
                                    code: session.code.clone(),
                                    slot_index,
                                },
                            );

                            if matches!(session.phase, LobbyPhase::InGame) {
                                // Partie déjà en cours → envoyer direct en jeu
                                net.send_lobby(
                                    client_id,
                                    &LobbyMessage::GameStarting { countdown_secs: 0 },
                                );

                                if let Some(lives) = resources.get::<SharedLives>() {
                                    net.send_event(
                                        client_id,
                                        &GameEvent {
                                            kind: GameEventKind::SharedLivesUpdate {
                                                remaining: lives.remaining,
                                                max: lives.max,
                                            },
                                        },
                                    );
                                }
                            } else {
                                // Lobby normal
                                net.broadcast_lobby(&LobbyMessage::LobbyUpdate {
                                    slots: session.to_slot_infos(),
                                    phase: session.to_phase_info(),
                                });
                            }
                        }
                        None => {
                            let msg = LobbyMessage::SessionError {
                                reason: SessionErrorKind::SessionFull,
                            };
                            net.send_lobby(client_id, &msg);
                        }
                    }
                }
                ServerEvent::ClientDisconnected { client_id, .. } => {
                    println!("Client disconnected: {}", client_id);
                    session.remove_slot(client_id);

                    if let Some(entry) = resources
                        .get_mut::<PlayerRegistry>()
                        .unwrap()
                        .remove(client_id)
                    {
                        if let Some(entity) = entry.entity {
                            world.remove(entity);
                        }
                    }

                    net.broadcast_lobby(&LobbyMessage::LobbyUpdate {
                        slots: session.to_slot_infos(),
                        phase: session.to_phase_info(),
                    });
                }
            }
        }

        // Traite les inputs reçus du client et les stocke dans les ressources globales
        {
            for (client_id, packet) in net.drain_inputs() {
                if let Some(entity) = resources
                    .get::<PlayerRegistry>()
                    .unwrap()
                    .get_entity(client_id)
                {
                    apply_input(&mut world, entity, &packet);
                }
            }
        }

        {
            for (client_id, shop_action) in net.drain_shop_actions() {
                handle_shop_action(client_id, &mut net, shop_action, &mut resources);
            }
        }

        for (client_id, msg) in net.drain_lobby_messages() {
            lobby::handle_lobby_message(
                client_id,
                msg,
                &mut session,
                &mut net,
                &mut world,
                &mut resources,
            );
        }

        // Met à jour le tick actuel dans les ressources pour que les systèmes puissent y accéder
        if let Some(mut res_dt) = resources.get_mut::<Duration>() {
            *res_dt = TICK_DURATION; // timestep fixe côté serveur
        }

        schedule.execute(&mut world, &mut resources);

        {
            // On récupère temporairement les IDs des joueurs connectés
            let active_clients: Vec<u64> = resources
                .get::<PlayerRegistry>()
                .unwrap()
                .iter_clients()
                .collect();

            {
                let mut buff_manager = resources.get_mut::<BufferManager>().unwrap();
                // On génère et on envoie un snapshot dédié à chaque client
                for client_id in active_clients {
                    let (id, entities) = buff_manager
                        .acquire::<Vec<EntityState>>()
                        .expect("Buffer manager à échouer à donner un buffer");
                    build_entities(&world, entities);

                    let snapshot = StateSnapshot {
                        tick_id,
                        entities: std::mem::take(entities),
                        wave_info: build_wave_info(&resources),
                        player_info: build_player_info(client_id, &mut world, &resources),
                    };

                    net.send_snapshot(client_id, &snapshot);
                    buff_manager.release(id);
                }
            }

            {
                let mut game_events = resources
                    .get_mut::<Queue<GameEvent>>()
                    .expect("GameEventQueue pas dans les ressources");
                let mapping = resources
                    .get::<PlayerRegistry>()
                    .expect("EntityToClient pas dans les ressources");

                for event in game_events.data.drain(..) {
                    match event.kind {
                        GameEventKind::PlayerDied { entity_id } => {
                            if let Some(client_id) = mapping.entity_to_client(entity_id) {
                                println!("Envoi de la mort au client concerné : {}", client_id);
                                net.send_event(client_id, &event);

                                if let Some(entity) = resources
                                    .get::<PlayerRegistry>()
                                    .unwrap()
                                    .get_entity(client_id)
                                {
                                    if let Ok(mut entry) = world.entry_mut(entity) {
                                        if let Ok(active) = entry.get_component_mut::<Active>() {
                                            active.0 = false;
                                        }
                                        if let Ok(vel) = entry.get_component_mut::<Velocity>() {
                                            vel.dx = 0.0;
                                            vel.dy = 0.0;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            net.broadcast_event(&event);
                        }
                    }
                }
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
        if let Ok(active) = entry.get_component::<Active>() {
            if !active.0 {
                return;
            }
        }
        if let Ok(state) = entry.get_component_mut::<InputState>() {
            state.move_dir = packet.move_dir;
            state.aim_dir = packet.aim_dir;
            state.dash = packet.dash;
            state.spell = packet.spell;
            state.attack = packet.attack;
        }
    }
}

fn handle_shop_action(
    client: u64,
    server: &mut GameNetServer,
    action: ShopAction,
    res: &mut Resources,
) {
    match action.kind {
        ShopActionKind::Open => {
            println!("Client {} à ouvert le shop", client);

            let shop_inventory = {
                let item_pool = res.get::<ItemPool>().unwrap();
                let mut player_shops = res.get_mut::<PlayerShops>().unwrap();
                player_shops.generate(client, &item_pool.items)
            };
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

            let gold = res.get::<PlayerRegistry>().unwrap().get_gold(client);
            let item = {
                let mut player_shop = res.get_mut::<PlayerShops>().unwrap();
                player_shop.buy(client, action.slot as usize, gold)
            };

            match item {
                Some(item) => {
                    println!(
                        "Client {} as acheter l'item du slot {}",
                        client, action.slot
                    );
                    res.get_mut::<PlayerRegistry>()
                        .unwrap()
                        .sub_gold(client, item.price);
                    server.send_event(
                        client,
                        &GameEvent {
                            kind: GameEventKind::ItemBought {
                                slot: action.slot as usize,
                            },
                        },
                    );
                }
                None => {
                    println!(
                        "Client {} n'a pas pu acheter l'item du slot {}",
                        client, action.slot
                    );
                    server.send_event(
                        client,
                        &GameEvent {
                            kind: GameEventKind::PurchaseFailed {
                                slot: action.slot as usize,
                            },
                        },
                    );
                }
            }
        }
        ShopActionKind::Close => {
            println!("Client {} à fermer le shop", client);
        }
    }
}

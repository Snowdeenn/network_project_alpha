mod config;
mod lobby;
mod net;
mod session;
mod simulation;
mod snapshot;

use crate::config::*;
use crate::session::*;
use crate::simulation::helper::{PlayerHp, PlayerPos};
use crate::simulation::systems::{
    attack::*, coin::*, debug::*, health::*, ia::*, physics::*, state::dash_system, wave::*,
};

use legion::{Entity, EntityStore, IntoQuery, Resources, Schedule, component, world::World};
use net::server::GameNetServer;
use renet::ServerEvent;
use shared::config::{ClassConfig, ClassRegistery, GameConfig, PlayerClass};
use shared::protocol::*;
use simulation::shop::PlayerShops;
use simulation::{
    components::*, eco::*, event::*, helper::clear_resource_queues, input::InputQueue, wave::*,
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
// Une simple table : Key = EntityId (jeu), Value = client_id (réseau)
pub struct EntityToClient(pub HashMap<u64, u64>);

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1000);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut net = GameNetServer::new();
    let mut world = World::default();
    let mut resources = Resources::default();
    let players_entities: HashMap<u64, Entity> = HashMap::new();
    let mut client_ids: Vec<u64> = Vec::new();

    // --- resources ---
    {
        resources.insert(Duration::new(0, 0));
        resources.insert(InputQueue(vec![]));
        resources.insert(InputState::default());
        resources.insert(DamageQueue(vec![]));
        resources.insert(EnemyDiedQueue(vec![]));
        resources.insert(CoinSpawnQueue(vec![]));
        resources.insert(PickupQueue(vec![]));
        resources.insert(GameState::Playing);
        resources.insert(PlayerPos { x: 0.0, y: 0.0 });
        resources.insert(PlayerHp {
            hp: 100.0,
            max_hp: 100.0,
        });
        resources.insert(GameEventQueue(vec![]));
        resources.insert(PlayerShops::new());
        resources.insert(PlayerGold::new());
        resources.insert(players_entities);
        resources.insert(EntityToClient(HashMap::new()));
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
                    hp: 0,
                    max_hp: 0,
                    state: HealthState::Alive,
                },
                Active(false),
                AttackTimer {
                    remaining: Duration::ZERO,
                    interval: Duration::from_secs_f32(0.5),
                },
            ));

            let mut entry = world
                .entry(e)
                .expect("[Entry ennemi] Echec de la création de l'entry dans le main");
            entry.add_component(Target(None));
            entry.add_component(AttackStats {
                range: 0.0,
                damage: 0,
                box_half_length: 0.0,
                box_half_width: 0.0,
                projectile_speed: None,
            });
            entry.add_component(MovementStats {
                accel: 0.0,
                max_speed: 0.0,
            });
            pool.pool.push(e);
        }
        resources.insert(pool);
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
                    let game_cfg = resources.get::<GameConfig>().unwrap();
                    match session.add_slot(client_id, &game_cfg) {
                        Some(slot_index) => {
                            resources
                                .get_mut::<PlayerGold>()
                                .expect("PlayerGold est pas dans les resources")
                                .0
                                .insert(client_id, 0);

                            // Confirmer la jointure au client
                            let msg = LobbyMessage::SessionJoined {
                                code: session.code.clone(),
                                slot_index,
                            };
                            net.send_lobby(client_id, &msg);

                            // Broadcaster l'état du lobby à tous
                            let update = LobbyMessage::LobbyUpdate {
                                slots: session.to_slot_infos(),
                                phase: session.to_phase_info(),
                            };
                            net.broadcast_lobby(&update);
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
                    session.remove_slot(client_id);

                    // Cleanup existant
                    if let Some(entity) = resources
                        .get_mut::<HashMap<u64, Entity>>()
                        .unwrap()
                        .remove(&client_id)
                    {
                        if let Ok(entry) = world.entry_ref(entity) {
                            if let Ok(id) = entry.get_component::<EntityId>() {
                                resources
                                    .get_mut::<EntityToClient>()
                                    .unwrap()
                                    .0
                                    .remove(&id.0);
                            }
                        }
                        world.remove(entity);
                    }

                    resources
                        .get_mut::<PlayerGold>()
                        .unwrap()
                        .0
                        .remove(&client_id);

                    // Broadcaster le lobby mis à jour
                    let update = LobbyMessage::LobbyUpdate {
                        slots: session.to_slot_infos(),
                        phase: session.to_phase_info(),
                    };
                    net.broadcast_lobby(&update);
                }
            }
        }

        // Traite les inputs reçus du client et les stocke dans les ressources globales
        {
            for (client_id, packet) in net.drain_inputs() {
                if let Some(&entity) = resources
                    .get::<HashMap<u64, Entity>>()
                    .unwrap()
                    .get(&client_id)
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
            // On récupère temporairement les IDs des joueurs connectés
            let active_clients: Vec<u64> = {
                let players_entities = resources.get::<HashMap<u64, Entity>>().unwrap();
                players_entities.keys().cloned().collect()
            };

            // On génère et on envoie un snapshot dédié à chaque client
            for client_id in active_clients {
                let snapshot = build_snapshot(client_id, &mut world, &resources, tick_id);

                net.send_snapshot(client_id, &snapshot);
            }
        }

        {
            let mut game_events = resources
                .get_mut::<GameEventQueue>()
                .expect("GameEventQueue pas dans les ressources");
            let mapping = resources
                .get::<EntityToClient>()
                .expect("EntityToClient pas dans les ressources");

            for event in game_events.0.drain(..) {
                match event.kind {
                    GameEventKind::PlayerDied { entity_id } => {
                        if let Some(&client_id) = mapping.0.get(&entity_id) {
                            println!("Envoi de la mort au client concerné : {}", client_id);
                            net.send_event(client_id, &event);

                            if let Some(&entity) = resources
                                .get::<HashMap<u64, Entity>>()
                                .unwrap()
                                .get(&client_id)
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

            let gold_avaible = res.get::<PlayerGold>().unwrap();
            let item = {
                let mut player_shop = res.get_mut::<PlayerShops>().unwrap();
                player_shop.buy(client, action.slot as usize, gold_avaible.get(client))
            };
            drop(gold_avaible); // On libère le verrou sur res après utilisation de gold_avaible

            match item {
                Some(item) => {
                    println!(
                        "Client {} as acheter l'item du slot {}",
                        client, action.slot
                    );
                    if let Some(mut gold) = res.get_mut::<PlayerGold>() {
                        gold.sub(client, item.price);
                    }
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

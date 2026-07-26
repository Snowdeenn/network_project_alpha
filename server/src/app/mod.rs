// src/app/mod.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};

use legion::{Entity, EntityStore, Resources, Schedule, world::World};
use renet::ServerEvent;

use shared::buffer::BufferManager;
use shared::config::{ClassConfig, ClassRegistery, GameConfig, PlayerClass};
use shared::protocol::*;

use crate::core::config::*;
use crate::core::lobby;
use crate::net::server::GameNetServer;
use crate::core::player_registry::PlayerRegistry;
use crate::core::pool::{GamePools, PoolManager};
use crate::core::queue::Queue;
use crate::core::session::*;
use crate::core::simulation::systems::{
    attack::*, coin::*, debug::*, health::*, ia::*, physics::*, state::dash_system, wave::*,
};
use crate::core::simulation::{
    components::*, eco::*, event::*, helper::clear_resource_queues, shop::PlayerShops, wave::*,
};
use crate::net::snapshot::{build_entities, build_player_info, build_wave_info};
use crate::core::simulation::spatial_grid::SpatialGrid;

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

pub struct ServerApp {
    net: GameNetServer,
    world: World,
    resources: Resources,
    schedule: Schedule,
    session: SessionState,
    tick_id: u64,
    last: Instant,
}

impl ServerApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let net = GameNetServer::new();
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
            resources.insert(PlayerRegistry::with_capacity(16));
            resources.insert(BufferManager::with_capacity(24));
            resources.insert(SpatialGrid::new(128.0, 1920.0, 1080.0));
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
                wave_state: crate::core::simulation::wave::WaveState::Waiting,
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

        // ---- Pool Manager ----
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
                .expect("Le chemin ou les droits sur warrior.json ne sont pas bons");
            let assassin = std::fs::read_to_string("assets/classes/assassin.json")
                .expect("Le chemin ou les droits sur assassin.json ne sont pas bons");
            let mage = std::fs::read_to_string("assets/classes/mage.json")
                .expect("Le chemin ou les droits sur mage.json ne sont pas bons");
            let tank = std::fs::read_to_string("assets/classes/tank.json")
                .expect("Le chemin ou les droits sur tank.json ne sont pas bons");

            let p_w: ClassConfig =
                serde_json::from_str(&warrior).expect("Impossible de parser warrior.json");
            let p_a: ClassConfig =
                serde_json::from_str(&assassin).expect("Impossible de parser assassin.json");
            let p_m: ClassConfig =
                serde_json::from_str(&mage).expect("Impossible de parser mage.json");
            let p_t: ClassConfig =
                serde_json::from_str(&tank).expect("Impossible de parser tank.json");

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
                .expect("Le chemin ou les droits sur game_config.json ne sont pas bons");
            let game_config: GameConfig =
                serde_json::from_str(&json).expect("Impossible de parser game_config.json");

            resources.insert(SharedLives::new(game_config.shared_lives));
            resources.insert(game_config);
        }

        // --- Serveur Config ---
        {
            let json = std::fs::read_to_string("assets/config/server_config.json")
                .expect("Le chemin ou les droits sur server_config.json ne sont pas bons");
            let server_config: ServerConfig =
                serde_json::from_str(&json).expect("Impossible de parser server_config.json");

            resources.insert(server_config);
        }

        // --- Physics Config ---
        {
            let json = std::fs::read_to_string("assets/config/physics_config.json")
                .expect("Le chemin ou les droits sur physics_config.json ne sont pas bons");
            let physics_config: PhysicsConfig =
                serde_json::from_str(&json).expect("Impossible de parser physics_config.json");

            resources.insert(physics_config);
        }

        let schedule = Schedule::builder()
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

        let session = {
            let server_cfg = resources.get::<ServerConfig>().unwrap();
            SessionState::new(&server_cfg)
        };
        println!("Code de session : {}", session.code);
        println!("Serveur démarré sur 127.0.0.1:7777");

        Ok(Self {
            net,
            world,
            resources,
            schedule,
            session,
            tick_id: 0,
            last: Instant::now(),
        })
    }

    pub fn run(&mut self) {
        loop {
            let delta = self.last.elapsed();
            self.last = Instant::now();

            self.net.update(delta);

            for event in self.net.drain_events() {
                match event {
                    ServerEvent::ClientConnected { client_id } => {
                        println!("Client connected : {}", client_id);
                        let game_cfg = self.resources.get::<GameConfig>().unwrap();
                        match self.session.add_slot(client_id, &game_cfg) {
                            Some(slot_index) => {
                                self.resources
                                    .get_mut::<PlayerRegistry>()
                                    .unwrap()
                                    .add(client_id);

                                self.net.send_lobby(
                                    client_id,
                                    &LobbyMessage::SessionJoined {
                                        code: self.session.code.clone(),
                                        slot_index,
                                    },
                                );

                                if matches!(self.session.phase, LobbyPhase::InGame) {
                                    self.net.send_lobby(
                                        client_id,
                                        &LobbyMessage::GameStarting { countdown_secs: 0 },
                                    );

                                    if let Some(lives) = self.resources.get::<SharedLives>() {
                                        self.net.send_event(
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
                                    self.net.broadcast_lobby(&LobbyMessage::LobbyUpdate {
                                        slots: self.session.to_slot_infos(),
                                        phase: self.session.to_phase_info(),
                                    });
                                }
                            }
                            None => {
                                let msg = LobbyMessage::SessionError {
                                    reason: SessionErrorKind::SessionFull,
                                };
                                self.net.send_lobby(client_id, &msg);
                            }
                        }
                    }
                    ServerEvent::ClientDisconnected { client_id, .. } => {
                        println!("Client disconnected: {}", client_id);
                        self.session.remove_slot(client_id);

                        if let Some(entry) = self
                            .resources
                            .get_mut::<PlayerRegistry>()
                            .unwrap()
                            .remove(client_id)
                        {
                            if let Some(entity) = entry.entity {
                                self.world.remove(entity);
                            }
                        }

                        self.net.broadcast_lobby(&LobbyMessage::LobbyUpdate {
                            slots: self.session.to_slot_infos(),
                            phase: self.session.to_phase_info(),
                        });
                    }
                }
            }

            // Traite les inputs reçus du client
            {
                let mut buff_manager = self
                    .resources
                    .get_mut::<BufferManager>()
                    .expect("[Ressource] devrait retourner le BufferManager");
                let (input_id, inputs) = buff_manager
                    .acquire::<Vec<(u64, InputPacket)>>()
                    .expect("[BufferManager] devrait retourner un tuple id data");
                self.net.drain_inputs_into(inputs);
                for (client_id, packet) in inputs {
                    if let Some(entity) = self
                        .resources
                        .get::<PlayerRegistry>()
                        .unwrap()
                        .get_entity(*client_id)
                    {
                        apply_input(&mut self.world, entity, packet);
                    }
                }
                buff_manager.release(input_id);
            }

            // ---- Handle ShopAction ----
            {
                let actions: Vec<(u64, ShopAction)> = {
                    let mut buff_manager = self.resources.get_mut::<BufferManager>().unwrap();
                    let action_id = buff_manager.acquire_id::<Vec<(u64, ShopAction)>>();
                    let actions = buff_manager
                        .get_mut::<Vec<(u64, ShopAction)>>(action_id)
                        .unwrap();
                    self.net.drain_shop_actions_into(actions);
                    let owned = std::mem::take(actions);
                    buff_manager.release(action_id);
                    owned
                };
                for (client_id, shop_action) in actions {
                    handle_shop_action(client_id, &mut self.net, shop_action, &mut self.resources);
                }
            }

            for (client_id, msg) in self.net.drain_lobby_messages() {
                lobby::handle_lobby_message(
                    client_id,
                    msg,
                    &mut self.session,
                    &mut self.net,
                    &mut self.world,
                    &mut self.resources,
                );
            }

            if let Some(mut res_dt) = self.resources.get_mut::<Duration>() {
                *res_dt = TICK_DURATION;
            }

            self.schedule.execute(&mut self.world, &mut self.resources);

            // ---- Maj Snapshot ----
            {
                let active_clients: Vec<u64> = self
                    .resources
                    .get::<PlayerRegistry>()
                    .unwrap()
                    .iter_clients()
                    .collect();

                {
                    let mut buff_manager = self.resources.get_mut::<BufferManager>().unwrap();
                    for client_id in active_clients {
                        let (id, entities) = buff_manager
                            .acquire::<Vec<EntityState>>()
                            .expect("Buffer manager a échoué à donner un buffer");
                        build_entities(&self.world, entities);

                        let snapshot = StateSnapshot {
                            tick_id: self.tick_id,
                            entities: std::mem::take(entities),
                            wave_info: build_wave_info(&self.resources),
                            player_info: build_player_info(
                                client_id,
                                &mut self.world,
                                &self.resources,
                            ),
                        };

                        self.net.send_snapshot(client_id, &snapshot);
                        buff_manager.release(id);
                    }
                }

                {
                    let mut game_events = self
                        .resources
                        .get_mut::<Queue<GameEvent>>()
                        .expect("GameEventQueue pas dans les ressources");
                    let mapping = self
                        .resources
                        .get::<PlayerRegistry>()
                        .expect("EntityToClient pas dans les ressources");

                    for event in game_events.data.drain(..) {
                        match event.kind {
                            GameEventKind::PlayerDied { entity_id } => {
                                if let Some(client_id) = mapping.entity_to_client(entity_id) {
                                    println!("Envoi de la mort au client concerné : {}", client_id);
                                    self.net.send_event(client_id, &event);

                                    if let Some(entity) = self
                                        .resources
                                        .get::<PlayerRegistry>()
                                        .unwrap()
                                        .get_entity(client_id)
                                    {
                                        if let Ok(mut entry) = self.world.entry_mut(entity) {
                                            if let Ok(active) = entry.get_component_mut::<Active>()
                                            {
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
                                self.net.broadcast_event(&event);
                            }
                        }
                    }
                }
            }

            self.net.flush();
            clear_resource_queues(&mut self.resources);

            self.tick_id += 1;
            {
                let elapsed = self.last.elapsed();
                if elapsed < TICK_DURATION {
                    std::thread::sleep(TICK_DURATION - elapsed);
                }
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
            println!("Client {} a ouvert le shop", client);

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
            println!("Client {} a acheté un item du shop", client);

            let gold = res.get::<PlayerRegistry>().unwrap().get_gold(client);
            let item = {
                let mut player_shop = res.get_mut::<PlayerShops>().unwrap();
                player_shop.buy(client, action.slot as usize, gold)
            };

            match item {
                Some(item) => {
                    println!("Client {} a acheté l'item du slot {}", client, action.slot);
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
            println!("Client {} a fermé le shop", client);
        }
    }
}

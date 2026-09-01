use std::collections::HashMap;
use std::time::{Duration, Instant};

use legion::{Entity, Resources, Schedule, world::World};

use utils::buffer::BufferManager;
use utils::config::{ClassConfig, ClassRegistery, GameConfig, PlayerClass};
use utils::protocol::*;

use crate::navigation::FlowFieldManager;
use crate::navigation::SpatialGrid;
use crate::net::server::GameNetServer;
use crate::replication::event::*;
use crate::session::PlayerRegistry;
use crate::session::lobby;
use crate::session::session::*;
use utils::spell_types::RawSpell;
use crate::simulation::resources::{
    components::{self, *},
    shop::*,
    wave::*,
};
use crate::simulation::systems::flow_field::update_flow_fields_system;
use crate::simulation::systems::spawn::respawn_player_system;
use crate::simulation::systems::spells::{apply_aoe_system, listen_spell_cast_system, spell_cast_resolver_system, start_spell_cooldown_system, update_spell_cooldowns_system};
use crate::simulation::systems::{
    attack::*, coin::*, debug::*, health::*, ia::*, physics::*, state::dash_system, wave::*,
};
use crate::utils::{GamePools, PoolManager, Queue};
use crate::{config::*, simulation};

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
    last_tick: Instant,
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
                wave_state: crate::simulation::resources::wave::WaveState::Waiting,
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
            let items_json = std::fs::read_to_string("assets/config/spell.json")?;
            let items: Vec<Option<RawSpell>> = serde_json::from_str(&items_json)?;
            resources.insert(SpellPool { items } );
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

        // --- Map Grid --
        {
            let grid = {
                let game_config = resources.get::<GameConfig>().unwrap();
                let cell_size = 64.0;
                let grid_w = (game_config.arena_w as f32 / cell_size).ceil() as u32;
                let grid_h = (game_config.arena_h as f32 / cell_size).ceil() as u32;

                let generator = utils::map::generator::Generator::new(42, grid_w, grid_h);
                generator.generate()
                // utils::map::grid::Grid::new(grid_w, grid_h, 64.0)
            };
            resources.insert(grid);
        }

        // --- Spatial Grid ---
        {
            let spatial_grid = {
                let game_config = resources.get::<GameConfig>().unwrap();
                SpatialGrid::new(128.0, game_config.arena_w, game_config.arena_h)
            };
            resources.insert(spatial_grid);
        }

        // --- FlowFieldManager ---
        {
            let flow_field_manager = FlowFieldManager::default();
            resources.insert(flow_field_manager);
        }

        // --- ThreadPool ---
        {
            let thread_pool = weave::ThreadPoolBuidler::new()
                .num_thread(num_cpus::get())
                .thread_name("ThreadPool Server")
                .build();
            resources.insert(thread_pool);
        }

        // --- SpellResgister ---
        {
            let spell_register =
                simulation::resources::spells::SpellRegister::init("assets/config/spell.json")?;
            resources.insert(spell_register);
        }

        let schedule = Schedule::builder()
            .add_system(update_spell_cooldowns_system())
            .add_system(ia_targeting_system())
            .add_system(update_flow_fields_system())
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
            .add_system(listen_spell_cast_system())
            .add_system(spell_cast_resolver_system())
            .add_system(read_player_attack_intent_system())
            .add_system(ia_attack_system())
            .add_system(create_attack_box_system())
            .add_system(check_collide_attackbox_system())
            .add_system(kamikaze_suicide_system())
            .add_system(apply_damage_system())
            .add_system(apply_aoe_system())
            .add_system(start_spell_cooldown_system())
            .add_system(health_system())
            .add_system(coin_push_to_queue_system())
            .add_system(coin_spawn_system())
            .add_system(coin_pickup_system())
            .add_system(apply_pickup_system())
            .add_system(wave_death_reaper_system())
            .add_system(wave_spawner_system())
            .add_system(wave_flow_manager_system())
            .add_system(respawn_player_system())
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
            last_tick: Instant::now(),
        })
    }

    pub fn run(&mut self) {
        let target_dt = Duration::from_secs_f32(1.0 / 20.0);

        loop {
            let start = Instant::now();
            let dt = self.last_tick.elapsed();
            self.last_tick = start;

            self.tick(dt);

            // Régulation du rythme (60 Hz)
            let elapsed = start.elapsed();
            if elapsed < target_dt {
                std::thread::sleep(target_dt - elapsed);
            }
        }
    }

    fn tick(&mut self, dt: Duration) {
        self.net.update(dt);
        crate::net::poll_event(
            &mut self.net,
            &mut self.resources,
            &mut self.session,
            &mut self.world,
        );
        crate::replication::process_incoming_game_event(&mut self.net, &mut self.resources);

        // Traite les inputs reçus du client
        {
            use crate::simulation::resources::components;
            let (mut sub_world, _) = self
                .world
                .split::<(&components::Active, &mut components::InputState)>();
            crate::simulation::resources::process_input(
                &mut self.net,
                &mut self.resources,
                &mut sub_world,
            );
        }
        crate::simulation::resources::process_shop_action(&mut self.net, &mut self.resources);

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

        crate::simulation::run_simulation(&mut self.schedule, &mut self.world, &mut self.resources);
        // Process Game Event
        {
            let (mut world, _) = self
                .world
                .split::<(&components::Active, &mut components::Velocity)>();
            crate::replication::process_game_event(&mut self.net, &mut self.resources, &mut world);
        }

        // ---- Maj Snapshot ----
        {
            let active_clients: Vec<u64> = self
                .resources
                .get::<PlayerRegistry>()
                .unwrap()
                .iter_clients()
                .collect();
            crate::replication::send_snapshots_to(
                active_clients,
                &mut self.net,
                &self.resources,
                &mut self.world,
                self.tick_id,
            );
        }

        self.net.flush();
        crate::utils::clear_resource_queues(&mut self.resources);
    }
}

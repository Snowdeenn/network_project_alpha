//! JSON-lines driver for Python scenarios; uses production ECS systems, no UDP/GPU.
use legion::{Entity, EntityStore, IntoQuery, Resources, Schedule, World};
use serde::Deserialize;
use serde_json::{Value, json};
use server::{
    navigation::SpatialGrid,
    replication::{DamageEvent, TargetedGameEvent},
    session::PlayerRegistry,
    simulation::{
        resources::{components::*, spells::SpellRegister},
        systems::{attack::*, physics::*, spells::*},
    },
    utils::Queue,
};
use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
    time::Duration,
};
use utils::{
    buffer::BufferManager,
    config::{ClassConfig, ClassRegistery, PlayerClass},
    protocol::{GameEvent, SpellSlot},
};
use server::simulation::systems::health::apply_damage_system;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Command {
    Player {
        id: u64,
        class: PlayerClass,
        position: [f64; 2],
    },
    Enemy {
        id: u64,
        position: [f64; 2],
        hp: u32,
    },
    Equip {
        id: u64,
        spell: String,
        slot: usize,
    },
    Gold {
        id: u64,
        amount: u32,
    },
    Input {
        id: u64,
        #[serde(default)]
        movement: [f32; 2],
        aim: [f32; 2],
        slot: Option<usize>,
    },
    Step {
        ticks: u32,
    },
    Snapshot,
}

struct Simulation {
    world: World,
    resources: Resources,
    schedule: Schedule,
    entities: HashMap<u64, Entity>,
    tick: u64,
    events: Vec<Value>,
}

impl Simulation {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut resources = Resources::default();
        let mut classes = HashMap::new();
        for name in ["warrior", "assassin", "mage", "tank"] {
            let class: ClassConfig = serde_json::from_str(&std::fs::read_to_string(format!(
                "assets/classes/{name}.json"
            ))?)?;
            classes.insert(class.class, class);
        }
        resources.insert(ClassRegistery { config: classes });
        resources.insert(SpellRegister::init("assets/config/spell.json")?);
        resources.insert(PlayerRegistry::with_capacity(4));
        resources.insert(Duration::from_millis(50));
        resources.insert(BufferManager::with_capacity(24));
        resources.insert(SpatialGrid::new(128.0, 9600.0, 6400.0));
        resources.insert(Queue::<DamageEvent> { data: vec![] });
        resources.insert(Queue::<GameEvent> { data: vec![] });
        resources.insert(Queue::<TargetedGameEvent> { data: vec![] });
        resources.insert(Queue::<SpellSupportEvent> { data: vec![] });
        resources.insert(ActiveBurns::default());
        resources.insert(ActiveSlows::default());
        let schedule = Schedule::builder()
            .add_system(update_spell_cooldowns_system())
            .add_system(friction_system())
            .add_system(update_velocity_system())
            .add_system(update_active_slows_system())
            .add_system(knockback_system())
            .add_system(update_position_system())
            .add_system(projectile_life_time_system())
            .add_system(listen_spell_cast_system())
            .flush()
            .add_system(spell_cast_resolver_system())
            .flush()
            .add_system(check_collide_attackbox_system())
            .flush()
            .add_system(apply_aoe_system())
            .add_system(apply_effect_system())
            .add_system(apply_support_effects_system())
            .add_system(update_active_burns_system())
            .add_system(apply_damage_system())
            .build();
        Ok(Self {
            world: World::default(),
            resources,
            schedule,
            entities: HashMap::new(),
            tick: 0,
            events: vec![],
        })
    }

    fn entity(&self, id: u64) -> Result<Entity, String> {
        self.entities
            .get(&id)
            .copied()
            .ok_or_else(|| format!("Unknown entity {id}"))
    }

    fn execute(&mut self, command: Command) -> Result<Value, String> {
        match command {
            Command::Player {
                id,
                class,
                position,
            } => {
                if self.entities.contains_key(&id) {
                    return Err("Duplicate entity ID".into());
                }
                if self
                    .resources
                    .get::<PlayerRegistry>()
                    .unwrap()
                    .iter_clients()
                    .count()
                    >= 4
                {
                    return Err("Maximum four players".into());
                }
                let entity = server::simulation::systems::spawn::spawn_player(
                    &mut self.world,
                    server::app::next_id(),
                    &self.resources.get::<ClassRegistery>().unwrap(),
                    class,
                    Position {
                        x: position[0],
                        y: position[1],
                    },
                );
                let entity_id = self
                    .world
                    .entry_ref(entity)
                    .unwrap()
                    .get_component::<EntityId>()
                    .unwrap()
                    .0;
                let mut players = self.resources.get_mut::<PlayerRegistry>().unwrap();
                players.add(id);
                players.link_entity(id, entity, entity_id);
                self.entities.insert(id, entity);
            }
            Command::Enemy { id, position, hp } => {
                if self.entities.contains_key(&id) || hp == 0 {
                    return Err("Duplicate ID or zero health".into());
                }
                let entity = self.world.push((
                    EntityId(server::app::next_id()),
                    IA,
                    Position {
                        x: position[0],
                        y: position[1],
                    },
                    Velocity::default(),
                    Collider { w: 40.0, h: 40.0 },
                    Health {
                        hp,
                        max_hp: hp,
                        state: HealthState::Alive,
                    },
                    Active(true),
                ));
                self.entities.insert(id, entity);
            }
            Command::Equip { id, spell, slot } => {
                if slot >= 4 {
                    return Err("Invalid spell slot".into());
                }
                let registry = self.resources.get::<SpellRegister>().unwrap();
                let spell_id = *registry.resolve_string(&spell).ok_or("Unknown spell")?;
                let charges = registry.get_spell(spell_id).unwrap().cast_cost.charges;
                if !self
                    .resources
                    .get_mut::<PlayerRegistry>()
                    .unwrap()
                    .add_spell(id, spell_id, SpellSlot::from(slot), charges)
                {
                    return Err("Unknown player or occupied slot".into());
                }
            }
            Command::Gold { id, amount } => {
                let mut players = self.resources.get_mut::<PlayerRegistry>().unwrap();
                if players.get_entry(id).is_none() {
                    return Err("Unknown player".into());
                }
                players.add_gold(id, amount);
            }
            Command::Input {
                id,
                movement,
                aim,
                slot,
            } => {
                if slot.is_some_and(|s| s >= 4) {
                    return Err("Invalid spell slot".into());
                }
                let entity = self.entity(id)?;
                let mut entry = self.world.entry_mut(entity).map_err(|e| e.to_string())?;
                let input = entry
                    .get_component_mut::<InputState>()
                    .map_err(|e| e.to_string())?;
                input.move_dir = movement;
                let length = (aim[0] * aim[0] + aim[1] * aim[1]).sqrt();
                if !length.is_finite() || length <= 0.0 {
                    return Err("Aim must be finite and nonzero".into());
                }
                input.aim_dir = [aim[0] / length, aim[1] / length];
                input.spell = slot.map(SpellSlot::from);
            }
            Command::Step { ticks } => {
                if ticks > 10000 {
                    return Err("Maximum 10000 ticks per command".into());
                }
                for _ in 0..ticks {
                    self.schedule.execute(&mut self.world, &mut self.resources);
                    self.tick += 1;
                    for event in self
                        .resources
                        .get_mut::<Queue<GameEvent>>()
                        .unwrap()
                        .data
                        .drain(..)
                    {
                        self.events.push(
                            json!({"tick": self.tick, "client_id": null, "kind": event.kind}),
                        );
                    }
                    for event in self
                        .resources
                        .get_mut::<Queue<TargetedGameEvent>>()
                        .unwrap()
                        .data
                        .drain(..)
                    {
                        self.events.push(json!({"tick": self.tick, "client_id": event.client_id, "kind": event.event.kind}));
                    }
                    self.resources
                        .get_mut::<Queue<DamageEvent>>()
                        .unwrap()
                        .data
                        .clear();
                }
            }
            Command::Snapshot => return Ok(self.snapshot()),
        }
        Ok(json!({"tick": self.tick}))
    }

    fn snapshot(&mut self) -> Value {
        let players = self.resources.get::<PlayerRegistry>().unwrap();
        let mut entities = serde_json::Map::new();
        for (id, entity) in &self.entities {
            if let Ok(entry) = self.world.entry_ref(*entity) {
                let health = entry.get_component::<Health>().unwrap();
                let position = entry.get_component::<Position>().unwrap();
                entities.insert(id.to_string(), json!({"hp": health.hp, "max_hp": health.max_hp,
                    "position": [position.x, position.y], "gold": players.get_entry(*id).map(|p| p.gold),
                    "cooldowns": players.cooldowns(*id)}));
            }
        }
        let projectiles = <&Projectile>::query().iter(&self.world).count();
        json!({"tick": self.tick, "entities": entities, "projectiles": projectiles, "events": std::mem::take(&mut self.events)})
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut simulation = Simulation::new()?;
    for line in io::stdin().lock().lines() {
        let response = serde_json::from_str::<Command>(&line?)
            .map_err(|e| e.to_string())
            .and_then(|command| simulation.execute(command));
        let output = match response {
            Ok(data) => json!({"ok": true, "data": data}),
            Err(error) => json!({"ok": false, "error": error}),
        };
        println!("{output}");
        io::stdout().flush()?;
    }
    Ok(())
}

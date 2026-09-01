use crate::app::next_id;
use crate::session::PlayerRegistry;
use crate::simulation::resources::components::*;
use crate::utils::Queue;
use legion::world::{EntryMut, World};
use legion::*;
use std::f64::consts::PI;
use std::time::Duration;
use utils::config::ClassRegistery;
use utils::protocol::GameEvent;

use crate::simulation::resources::wave::EnemyStatsConfig;

pub fn spawn_player(
    world: &mut World,
    player_game_id: u64,
    registry: &ClassRegistery,
    class: utils::config::PlayerClass,
    spawn_pos: Position,
) -> Entity {
    let config = registry
        .config
        .get(&class)
        .expect(&format!("Config pour la classe {:?}, n'existe pas", class));

    let entity = world.push((
        EntityId(player_game_id),
        Player,
        class,
        InputState::default(),
        spawn_pos,
        Velocity { dx: 0.0, dy: 0.0 },
        Dash(DashState::Idle),
        Collider {
            h: config.collider.h,
            w: config.collider.w,
        },
    ));

    let mut entry = world
        .entry(entity)
        .expect("[Spawner] Impossible de créer l'entry");
    entry.add_component(Active(true));
    entry.add_component(AttackTimer {
        remaining: Duration::ZERO,
        interval: Duration::from_secs_f64(config.attack_interval_secs),
    });
    entry.add_component(MovementStats {
        max_speed: config.movement.max_speed,
        accel: config.movement.accel,
    });
    entry.add_component(AttackStats {
        range: config.attack.range,
        damage: config.attack.damage,
        box_half_length: config.attack.box_half_length,
        box_half_width: config.attack.box_half_width,
        projectile_speed: config.attack.projectile_speed,
    });
    entry.add_component(Health {
        hp: config.max_hp,
        max_hp: config.max_hp,
        state: HealthState::Alive,
    });

    entity
}

pub fn spawn_enemy_blank(world: &mut World) -> Entity {
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
    entry.add_component(SpellCooldowns {
        slots: [f32::default(); 4],
    });
    e
}

pub fn spawn_coin_blank(world: &mut World) -> Entity {
    let e = world.push((
        EntityId(next_id()),
        Coin,
        Position { x: 0.0, y: 0.0 },
        Collider { w: 20.0, h: 20.0 },
        Active(false),
        CoinValue(0),
    ));
    e
}

const MAP_CENTER_X: f64 = 4800.0;
const MAP_CENTER_Y: f64 = 3200.0;
const SPAWN_RADIUS: f64 = 800.0;

pub fn configure_enemy(
    entry: &mut EntryMut,
    config: &EnemyStatsConfig,
    base_hp: u32,
    base_speed: f64,
) {
    if let Ok(id) = entry.get_component_mut::<EntityId>() {
        *id = EntityId(crate::app::next_id());
    }

    if let Ok(pos) = entry.get_component_mut::<Position>() {
        let angle = rand::random::<f64>() * 2.0 * PI;
        pos.x = MAP_CENTER_X + angle.cos() * SPAWN_RADIUS;
        pos.y = MAP_CENTER_Y + angle.sin() * SPAWN_RADIUS;
    }

    if let Ok(target) = entry.get_component_mut::<Target>() {
        target.0 = None;
    }
    if let Ok(health) = entry.get_component_mut::<Health>() {
        health.hp = (base_hp as f64 * config.hp_modifier) as u32;
        health.max_hp = (base_hp as f64 * config.hp_modifier) as u32;
        health.state = HealthState::Alive;
    }

    if let Ok(speed) = entry.get_component_mut::<MovementStats>() {
        speed.accel = base_speed * config.speed_modifier;
        speed.max_speed = config.max_speed;
    }

    if let Ok(attack_stats) = entry.get_component_mut::<AttackStats>() {
        attack_stats.range = config.range;
        attack_stats.damage = config.damage;
        attack_stats.projectile_speed = config.projectile_speed;
        attack_stats.box_half_length = config.box_half_length;
        attack_stats.box_half_width = config.box_half_width;
    }
}

#[system]
#[write_component(Active)]
#[write_component(Position)]
#[write_component(Health)]
pub fn respawn_player(
    world: &mut legion::world::SubWorld,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
    #[resource] registry: &PlayerRegistry,
    #[resource] class_registry: &ClassRegistery,
) {
    let to_respawn: Vec<u64> = game_event_queue
        .iter()
        .filter_map(|event| match event.kind {
            utils::protocol::GameEventKind::RespawnPlayer { client_id } => Some(client_id),
            _ => None,
        })
        .collect();

    for client_id in to_respawn {
        let Some(entity) = registry.get_entity(client_id) else {
            tracing::error!("Échec de l'acquisition de l'entity pour le client : {client_id}");
            continue;
        };
        let Ok(mut entity_entry) = world.entry_mut(entity) else {
            tracing::error!("Echec lors de la création de l'entry de l'entity : {entity:?}");
            continue;
        };

        if let Ok(active) = entity_entry.get_component_mut::<Active>() {
            active.0 = true;
        }
        if let Ok(pos) = entity_entry.get_component_mut::<Position>() {
            pos.x = MAP_CENTER_X;
            pos.y = MAP_CENTER_Y;
        }
        if let Ok(health) = entity_entry.get_component_mut::<Health>() {
            let class_config = class_registry
                .config
                .get(&utils::config::PlayerClass::Warrior)
                .unwrap();
            health.hp = class_config.max_hp;
            health.max_hp = class_config.max_hp;
            health.state = HealthState::Alive;
        }

        game_event_queue.push(GameEvent {
            kind: utils::protocol::GameEventKind::RespawnAccept { client_id },
        });
    }
}

use crate::simulation::components::*;
use legion::world::World;
use legion::*;
use shared::ClassRegistery;
use std::time::Duration;

pub fn spawn_player(
    world: &mut World,
    player_game_id: u64,
    registry: &ClassRegistery,
    class: shared::PlayerClass,
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

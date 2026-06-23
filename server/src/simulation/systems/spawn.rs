use legion::*;
use legion::world::World;
use std::time::Duration;
use crate::simulation::components::*;

pub fn spawn_player(
    world: &mut World,
    player_game_id: u64,
    class: PlayerClass,
    spawn_pos: Position,
) -> Entity {
    let (max_hp, collider, move_stats, attack_stats, attack_interval) = match class {
        PlayerClass::Warrior => (
            100,
            Collider { w: 40.0, h: 40.0 },
            MovementStats {
                accel: 1500.0,
                max_speed: 300.0,
            },
            AttackStats {
                range: 60.0,
                damage: 15,
                box_half_length: 25.0,
                box_half_width: 30.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.5),
        ),
        PlayerClass::Assassin => (
            75,
            Collider { w: 32.0, h: 32.0 },
            MovementStats {
                accel: 2200.0,
                max_speed: 400.0,
            },
            AttackStats {
                range: 50.0,
                damage: 25,
                box_half_length: 20.0,
                box_half_width: 20.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.3),
        ),
        PlayerClass::Mage => (
            80,
            Collider { w: 36.0, h: 36.0 },
            MovementStats {
                accel: 1200.0,
                max_speed: 250.0,
            },
            AttackStats {
                range: 300.0,
                damage: 18,
                box_half_length: 15.0,
                box_half_width: 15.0,
                projectile_speed: Some(400.0),
            },
            Duration::from_secs_f32(0.6),
        ),
        PlayerClass::Tank => (
            180,
            Collider { w: 48.0, h: 48.0 },
            MovementStats {
                accel: 900.0,
                max_speed: 200.0,
            },
            AttackStats {
                range: 55.0,
                damage: 10,
                box_half_length: 30.0,
                box_half_width: 45.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.7),
        ),
    };

    let entity = world.push((
        EntityId(player_game_id),
        Player,
        class,
        InputState::default(),
        spawn_pos,
        Velocity { dx: 0.0, dy: 0.0 },
        Dash(DashState::Idle),
        collider,
    ));

    let mut entry = world
        .entry(entity)
        .expect("[Spawner] Impossible de créer l'entry");
    entry.add_component(Active(true));
    entry.add_component(AttackTimer {
        remaining: Duration::ZERO,
        interval: attack_interval,
    });
    entry.add_component(move_stats);
    entry.add_component(attack_stats);
    entry.add_component(Health {
        hp: max_hp,
        max_hp,
        state: HealthState::Alive,
    });

    entity
}
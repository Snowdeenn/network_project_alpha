use crate::next_id;
use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::helper::obb_vs_aabb;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::protocol::{GameEvent, GameEventKind};
use std::time::Duration;

#[system]
#[read_component(Player)]
#[read_component(InputState)]
#[read_component(AttackStats)]
#[write_component(AttackTimer)]
pub fn read_player_attack_intent(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] dt: &Duration,
) {
    let mut query = <(Entity, &InputState, &AttackStats, &mut AttackTimer)>::query()
        .filter(component::<Player>());

    for (entity, state, stats, timer) in query.iter_mut(world) {
        timer.remaining = timer.remaining.saturating_sub(*dt);

        if state.attack && timer.remaining.is_zero() {
            command.add_component(
                *entity,
                AttackIntent {
                    aim_dir: state.aim_dir,
                    box_half_length: stats.box_half_length,
                    box_half_width: stats.box_half_width,
                    projectile_speed: stats.projectile_speed,
                    damage: stats.damage,
                    range: stats.range,
                },
            );
            timer.remaining = timer.interval;
        }
    }
}

#[system]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Target)]
#[read_component(AttackStats)]
#[write_component(AttackTimer)]
pub fn ia_attack(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] dt: &Duration,
) {
    let player_positions: std::collections::HashMap<Entity, Position> = {
        let mut player_query = <(Entity, &Position)>::query().filter(component::<Player>());
        player_query
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos))
            .collect()
    };

    let mut query = <(
        Entity,
        &Position,
        &Active,
        &AttackStats,
        &Target,
        &mut AttackTimer,
    )>::query()
    .filter(component::<IA>());

    for (entity, ia_pos, active, stats, target, timer) in query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        timer.remaining = timer.remaining.saturating_sub(*dt);

        if let Some(target_entity) = target.0 {
            if let Some(target_pos) = player_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < stats.range && timer.remaining.is_zero() {
                    command.add_component(
                        *entity,
                        AttackIntent {
                            aim_dir: [(dx / distance) as f32, (dy / distance) as f32],
                            box_half_length: stats.box_half_length,
                            box_half_width: stats.box_half_width,
                            projectile_speed: stats.projectile_speed,
                            damage: stats.damage,
                            range: stats.range,
                        },
                    );
                    timer.remaining = timer.interval;
                }
            }
        }
    }
}

const OFFSET_ATTACKBOX: f32 = 10.0;
const PLAYER_RADIUS: f32 = 20.0;

#[system(for_each)]
#[filter(component::<AttackIntent>())]
pub fn create_attack_box(
    entity: &Entity,
    pos: &Position,
    intent: &AttackIntent,
    command: &mut CommandBuffer,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    let dir = intent.aim_dir;

    if let Some(speed) = intent.projectile_speed {
        let entity = command.push((
            EntityId(next_id()),
            Position { x: pos.x, y: pos.y },
            Velocity {
                dx: dir[0] as f64 * speed,
                dy: dir[1] as f64 * speed,
            },
            Geometry {
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
            Damage(intent.damage),
            TeamFilter { is_player: true }, // Pour que tes IA prennent les dégâts
            Owner(*entity),
            Projectile,
        ));
        command.add_component(entity, Active(true));
        let life_time = intent.range / speed;
        command.add_component(entity, LifeTime(Duration::from_secs_f64(life_time)));
    } else {
        let dist_to_center =
            (PLAYER_RADIUS + OFFSET_ATTACKBOX + intent.box_half_length as f32) as f64;
        let center_x = pos.x + (dir[0] as f64 * dist_to_center);
        let center_y = pos.y + (dir[1] as f64 * dist_to_center);

        command.push((
            Position {
                x: center_x,
                y: center_y,
            },
            Geometry {
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
            Damage(intent.damage),
            TeamFilter { is_player: true },
            Owner(*entity), // On garde l'owner pour ton système actuel
            Active(true),
        ));

        // Rendu Debug via ton événement réseau existant !
        game_event_queue.0.push(GameEvent {
            kind: GameEventKind::DebugRect {
                x: center_x as f32,
                y: center_y as f32,
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
        });
    }

    command.remove_component::<AttackIntent>(*entity);
}

#[system]
#[read_component(Player)]
#[read_component(IA)]
#[read_component(Projectile)]
#[read_component(Collider)]
#[read_component(Geometry)]
#[read_component(Position)]
#[read_component(Owner)]
#[read_component(Health)]
#[read_component(Damage)]
pub fn check_collide_attackbox(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] damage_queue: &mut DamageQueue,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    let players: std::collections::HashSet<Entity> = <Entity>::query()
        .filter(component::<Player>())
        .iter(world)
        .copied()
        .collect();

    let mut attackbox_query = <(Entity, &Geometry, &Owner, &Damage, &Position)>::query();
    let attackboxes: Vec<_> = attackbox_query
        .iter(world)
        .map(|(e, g, o, d, p)| (*e, *g, *o, *d, *p))
        .collect();

    let mut victim_query = <(Entity, &Collider, &Position)>::query().filter(component::<Health>());
    let victims: Vec<_> = victim_query
        .iter(world)
        .map(|(e, c, p)| (*e, *c, *p))
        .collect();

    for (attackbox_entt, attackbox, owner, damage, attackbox_pos) in attackboxes {
        let attacker_is_player = players.contains(&owner.0);
        let is_projectile = world
            .entry_ref(attackbox_entt)
            .map(|e| e.get_component::<Projectile>().is_ok())
            .unwrap_or(false);
        let mut hit = false;

        for (victim_entt, victim_col, victim_pos) in &victims {
            if *victim_entt == owner.0 {
                continue;
            }
            if obb_vs_aabb(&attackbox_pos, &attackbox, victim_pos, victim_col) {
                let mut should_damage = false;

                if attacker_is_player {
                    should_damage = true;
                } else {
                    let victim_is_player = players.contains(victim_entt);
                    if victim_is_player {
                        should_damage = true;
                    }
                }

                if should_damage {
                    damage_queue.0.push(DamageEvent {
                        target: *victim_entt,
                        amount: damage.0,
                    });
                    game_event_queue.0.push(GameEvent {
                        kind: GameEventKind::EntityHit {
                            pos: [victim_pos.x as f32, victim_pos.y as f32],
                        },
                    });

                    {
                        let mut dx = victim_pos.x - attackbox_pos.x;
                        let mut dy = victim_pos.y - attackbox_pos.y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > 0.0 {
                            dx /= distance;
                            dy /= distance;
                        } else {
                            dx = 1.0;
                            dy = 0.0;
                        }

                        let knockback_force = 600.0f32;
                        let knockback_duration = 0.12;

                        command.add_component(
                            *victim_entt,
                            Knockback {
                                dx: dx as f32 * knockback_force,
                                dy: dy as f32 * knockback_force,
                                duration: knockback_duration,
                            },
                        );
                        hit = true;
                    }
                }
            }
        }

        if !is_projectile || hit {
            command.remove(attackbox_entt);
        }
    }
}

#[system(for_each)]
#[filter(component::<Knockback>())]
pub fn knockback(
    entt: &Entity,
    kb: &mut Knockback,
    velo: &mut Velocity,
    #[resource] dt: &Duration,
    command: &mut CommandBuffer,
) {
    velo.dx += kb.dx as f64;
    velo.dy += kb.dy as f64;

    const MAX_VELO: f64 = 600.0;
    velo.dx = velo.dx.clamp(-MAX_VELO, MAX_VELO);
    velo.dy = velo.dy.clamp(-MAX_VELO, MAX_VELO);

    kb.dx = 0.0;
    kb.dy = 0.0;
    kb.duration -= dt.as_secs_f32();

    if kb.duration <= 0.0 {
        command.remove_component::<Knockback>(*entt);
    }
}

const ARENA_W: f64 = 1920.0;
const ARENA_H: f64 = 1080.0;
#[system(for_each)]
#[filter(component::<Projectile>())]
pub fn projectile_life_time(
    entity: &Entity,
    pos: &Position,
    life: &mut LifeTime,
    command: &mut CommandBuffer,
    #[resource] dt: &Duration,
) {
    const MARGIN: f64 = 100.0;
    if pos.x < -MARGIN || pos.x > ARENA_W + MARGIN || pos.y < -MARGIN || pos.y > ARENA_H + MARGIN {
        command.remove(*entity);
    }
    let lt = life;
    let remaining = lt.0.saturating_sub(*dt);
    if remaining.is_zero() {
        command.remove(*entity);
    } else {
        lt.0 = remaining;
    }

}

use crate::app::next_id;
use crate::navigation::SpatialGrid;
use crate::replication::event::*;
use crate::simulation::resources::components::*;
use crate::simulation::systems::spells::ActiveBurns;
use crate::simulation::systems::spells::ActiveSlows;
use crate::utils::Queue;
use crate::utils::obb_vs_aabb;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use utils::buffer::BufferManager;
use utils::protocol::{GameEvent, GameEventKind};

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
    #[resource] buff_manager: &mut BufferManager,
) {
    let (p_position_id, p_positions) = buff_manager.acquire::<HashMap<Entity, Position>>().unwrap();
    let mut player_query = <(Entity, &Position)>::query().filter(component::<Player>());
    p_positions.extend(
        player_query
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos)),
    );

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
            if let Some(target_pos) = p_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (PLAYER_RADIUS as f64 + stats.range) && timer.remaining.is_zero() {
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
    buff_manager.release(p_position_id);
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
    #[resource] game_event_queue: &mut Queue<GameEvent>,
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
            Owner(*entity),
            Active(true),
        ));

        // Rendu Debug
        game_event_queue.data.push(GameEvent {
            kind: GameEventKind::SpawnRect {
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
#[read_component(SpellEffects)]
pub fn check_collide_attackbox(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] damage_queue: &mut Queue<DamageEvent>,
    #[resource] active_burns: &mut ActiveBurns,
    #[resource] active_slows: &mut ActiveSlows,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
    #[resource] buff_manager: &mut BufferManager,
    #[resource] grid: &mut SpatialGrid,
) {
    grid.clear();

    let players_id = buff_manager.acquire_id::<HashSet<Entity>>();
    let attackboxes_id =
        buff_manager.acquire_id::<Vec<(Entity, Geometry, Owner, Option<Damage>, Position)>>();
    let victims_id = buff_manager.acquire_id::<Vec<(Entity, Collider, Position)>>();
    let candidates_id = buff_manager.acquire_id::<Vec<usize>>();

    {
        let players = buff_manager
            .get_mut::<HashSet<Entity>>(players_id)
            .expect("[Buffer Manager] HashSet<Entity> introuvable");
        players.extend(
            <Entity>::query()
                .filter(component::<Player>())
                .iter(world)
                .copied(),
        );
    }
    {
        let attackboxes = buff_manager
            .get_mut::<Vec<(Entity, Geometry, Owner, Option<Damage>, Position)>>(attackboxes_id)
            .expect("[Buffer Manager] Vec<Attackbox> introuvable");
        attackboxes.extend(
            <(Entity, &Geometry, &Owner, Option<&Damage>, &Position)>::query()
                .iter(world)
                .map(|(e, g, o, d, p)| (*e, *g, *o, d.copied(), *p)),
        );
    }
    {
        let victims = buff_manager
            .get_mut::<Vec<(Entity, Collider, Position)>>(victims_id)
            .expect("[Buffer Manager] Vec<Victim> introuvable");
        victims.extend(
            <(Entity, &Collider, &Position)>::query()
                .filter(component::<Health>())
                .iter(world)
                .map(|(e, c, p)| (*e, *c, *p)),
        );

        for (idx, (_, col, pos)) in victims.iter().enumerate() {
            grid.insert(idx, pos, col);
        }
    }

    grid.build();

    let mut candidates = std::mem::take(
        buff_manager
            .get_mut::<Vec<usize>>(candidates_id)
            .expect("[Buffer Manager] Candidates introuvable"),
    );

    let players = buff_manager
        .get::<HashSet<Entity>>(players_id)
        .expect("[Buffer Manager] HashSet<Entity> introuvable");
    let attackboxes = buff_manager
        .get::<Vec<(Entity, Geometry, Owner, Option<Damage>, Position)>>(attackboxes_id)
        .expect("[Buffer Manager] Vec<Attackbox> introuvable");
    let victims = buff_manager
        .get::<Vec<(Entity, Collider, Position)>>(victims_id)
        .expect("[Buffer Manager] Vec<Victim> introuvable");

    for (attackbox_entt, attackbox_geom, owner, damage, attackbox_pos) in attackboxes.iter() {
        let attacker_is_player = players.contains(&owner.0);
        let is_projectile = world
            .entry_ref(*attackbox_entt)
            .map(|e| e.get_component::<Projectile>().is_ok())
            .unwrap_or(false);
        // --- BROADPHASE : Construire une AABB de recherche englobant l'OBB rotatée ---
        // Estimation conservatrice : un carré bordant basé sur la diagonale (w + h)
        let broadphase_w = (attackbox_geom.half_width + attackbox_geom.half_length) as f64;
        let broadphase_h = (attackbox_geom.half_width + attackbox_geom.half_length) as f64;
        let broadphase_pos = Position {
            x: attackbox_pos.x - broadphase_w * 0.5,
            y: attackbox_pos.y - broadphase_h * 0.5,
        };
        let broadphase_col = Collider {
            w: broadphase_w,
            h: broadphase_h,
        };

        candidates.clear();
        grid.query(&broadphase_pos, &broadphase_col, &mut candidates);
        candidates.sort_unstable();
        candidates.dedup();

        // --- NARROWPHASE : Tester uniquement les candidats retenus ---
        for &victim_idx in candidates.iter() {
            let (victim_entt, victim_col, victim_pos) = &victims[victim_idx];

            if *victim_entt == owner.0 {
                continue;
            }

            if obb_vs_aabb(attackbox_pos, attackbox_geom, victim_pos, victim_col) {
                let victim_is_player = players.contains(victim_entt);
                let should_damage = attacker_is_player != victim_is_player;

                if should_damage {
                    game_event_queue.data.push(GameEvent {
                        kind: GameEventKind::EntityHit {
                            pos: [victim_pos.x as f32, victim_pos.y as f32],
                        },
                    });

                    let spell_effects = world.entry_ref(*attackbox_entt).ok().and_then(|e| {
                        e.get_component::<SpellEffects>()
                            .ok()
                            .map(|se| (se.effects.clone(), se.aoe))
                    });
                    if let Some((effects, aoe)) = spell_effects {
                        match aoe {
                            Some(aoe) => {
                                command.push((
                                    PendingAoe {
                                        origin: [
                                            attackbox_pos.x as f32 + attackbox_geom.half_length,
                                            attackbox_pos.y as f32 + attackbox_geom.half_width,
                                        ],
                                        aim_dir: attackbox_geom.dir,
                                        aoe: Some(aoe),
                                        effects,
                                        owner: owner.0,
                                        caster_is_player: attacker_is_player,
                                    },
                                    Active(true),
                                ));
                            }
                            None => {
                                crate::simulation::systems::spells::apply_effects(
                                    &effects,
                                    *victim_entt,
                                    [
                                        attackbox_pos.x as f32 + attackbox_geom.half_length,
                                        attackbox_pos.y as f32 + attackbox_geom.half_width,
                                    ],
                                    [victim_pos.x as f32, victim_pos.y as f32],
                                    command,
                                    damage_queue,
                                    active_burns,
                                    active_slows,
                                );
                            }
                        }
                    } else {
                        let Some(damage) = damage else {
                            tracing::warn!(?attackbox_entt, "Hitbox sans dégâts ni effets de sort");
                            continue;
                        };
                        damage_queue.push(DamageEvent {
                            target: *victim_entt,
                            amount: damage.0,
                        });

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
                        command.add_component(
                            *victim_entt,
                            Knockback {
                                dx: dx as f32 * 600.0,
                                dy: dy as f32 * 600.0,
                                duration: 0.12,
                            },
                        );
                    }

                    if is_projectile {
                        command.remove(*attackbox_entt);
                        break;
                    }
                }
            }
        }

        // Une attaque de mêlée ne doit vivre qu'un tick, même si la broadphase
        // n'a trouvé aucun candidat. Les projectiles vivent jusqu'à un impact
        // ou jusqu'à l'expiration de leur LifeTime.
        if !is_projectile {
            command.remove(*attackbox_entt);
        }
    }
    buff_manager.release(players_id);
    buff_manager.release(attackboxes_id);
    buff_manager.release(victims_id);
    buff_manager.release(candidates_id);
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

const ARENA_W: f64 = 9600.0;
const ARENA_H: f64 = 6400.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::systems::health::apply_damage_system;
    use crate::simulation::systems::spells::apply_aoe_system;
    use legion::{EntityStore, Resources, Schedule, World};
    use utils::spell_types::{AoeSpellShape, Element, SpellEffectKind};

    fn test_resources() -> Resources {
        let mut resources = Resources::default();
        resources.insert(Queue::<DamageEvent> { data: vec![] });
        resources.insert(Queue::<GameEvent> { data: vec![] });
        resources.insert(BufferManager::with_capacity(8));
        resources.insert(SpatialGrid::new(64.0, 1_000.0, 1_000.0));
        resources
    }

    fn spawn_player(world: &mut World) -> Entity {
        world.push((Player,))
    }

    fn spawn_enemy(world: &mut World, x: f64, y: f64, hp: u32) -> Entity {
        world.push((
            IA,
            Position { x, y },
            Collider { w: 40.0, h: 40.0 },
            Health {
                hp,
                max_hp: hp,
                state: HealthState::Alive,
            },
            Active(true),
        ))
    }

    fn spawn_player_victim(world: &mut World, x: f64, y: f64, hp: u32) -> Entity {
        world.push((
            Player,
            Position { x, y },
            Collider { w: 40.0, h: 40.0 },
            Health {
                hp,
                max_hp: hp,
                state: HealthState::Alive,
            },
            Active(true),
        ))
    }

    fn spawn_spell_projectile(
        world: &mut World,
        owner: Entity,
        aoe: Option<AoeSpellShape>,
    ) -> Entity {
        world.push((
            Position { x: 20.0, y: 20.0 },
            Geometry {
                dir: [1.0, 0.0],
                half_length: 10.0,
                half_width: 10.0,
            },
            Owner(owner),
            Projectile,
            SpellEffects {
                effects: vec![SpellEffectKind::Damage {
                    amount: 10.0,
                    element: Element::Fire,
                }],
                aoe,
            },
        ))
    }

    fn health(world: &World, entity: Entity) -> u32 {
        world
            .entry_ref(entity)
            .unwrap()
            .get_component::<Health>()
            .unwrap()
            .hp
    }

    fn combat_schedule() -> Schedule {
        Schedule::builder()
            .add_system(check_collide_attackbox_system())
            .add_system(apply_aoe_system())
            .add_system(apply_damage_system())
            .build()
    }

    #[test]
    fn non_aoe_spell_applies_damage_once_to_the_direct_target() {
        let mut world = World::default();
        let owner = spawn_player(&mut world);
        let victim = spawn_enemy(&mut world, 20.0, 20.0, 100);
        let projectile = spawn_spell_projectile(&mut world, owner, None);
        let mut resources = test_resources();

        combat_schedule().execute(&mut world, &mut resources);

        assert_eq!(health(&world, victim), 90);
        assert!(world.entry_ref(projectile).is_err());
    }

    #[test]
    fn aoe_spell_applies_its_damage_once_to_the_impact_target() {
        let mut world = World::default();
        let owner = spawn_player(&mut world);
        let victim = spawn_enemy(&mut world, 20.0, 20.0, 100);
        spawn_spell_projectile(
            &mut world,
            owner,
            Some(AoeSpellShape::Circle {
                offset: utils::math::Vec2::zero(),
                radius: 50.0,
            }),
        );
        let mut resources = test_resources();

        let mut schedule = combat_schedule();
        schedule.execute(&mut world, &mut resources);
        // Le PendingAoe créé via CommandBuffer devient visible au tick suivant.
        schedule.execute(&mut world, &mut resources);

        assert_eq!(health(&world, victim), 90);
    }

    #[test]
    fn player_spell_does_not_damage_another_player() {
        let mut world = World::default();
        let owner = spawn_player(&mut world);
        let teammate = spawn_player_victim(&mut world, 20.0, 20.0, 100);
        spawn_spell_projectile(&mut world, owner, None);
        let mut resources = test_resources();

        combat_schedule().execute(&mut world, &mut resources);

        assert_eq!(health(&world, teammate), 100);
    }

    #[test]
    fn melee_attackbox_is_removed_even_without_candidates() {
        let mut world = World::default();
        let owner = spawn_player(&mut world);
        let attackbox = world.push((
            Position { x: 20.0, y: 20.0 },
            Geometry {
                dir: [1.0, 0.0],
                half_length: 10.0,
                half_width: 10.0,
            },
            Owner(owner),
            Damage(10),
        ));
        let mut resources = test_resources();
        let mut schedule = Schedule::builder()
            .add_system(check_collide_attackbox_system())
            .build();

        schedule.execute(&mut world, &mut resources);

        assert!(world.entry_ref(attackbox).is_err());
    }
}

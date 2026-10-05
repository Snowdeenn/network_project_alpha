use crate::{
    replication::{DamageEvent, TargetedGameEvent},
    session::{PlayerRegistry, SpellUseError},
    simulation::resources::components::*,
    simulation::resources::spells::SpellRegister,
};
use legion::{
    Entity, EntityStore, query::IntoQuery, system, systems::CommandBuffer, world::SubWorld,
};
use utils::protocol::GameEvent;
use utils::spell_types::*;

#[system]
#[write_component(InputState)]
#[read_component(EntityId)]
#[write_component(PendingSpellCast)]
pub fn listen_spell_cast(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    query: &mut legion::Query<(legion::Entity, &EntityId, &mut InputState)>,
    #[resource] player_registry: &mut PlayerRegistry,
    #[resource] spell_registry: &SpellRegister,
    #[resource] targeted_events: &mut crate::utils::Queue<TargetedGameEvent>,
) {
    for (entity, entity_id, input_state) in query.iter_mut(world) {
        let Some(client_id) = player_registry.entity_to_client(entity_id.0) else {
            tracing::warn!("L'entity n'est pas un joueur: EntityId => {entity_id:?}");
            continue;
        };
        // A spell input is an edge-triggered request. Consuming it prevents a lost
        // follow-up UDP packet from casting the same spell on every server tick.
        let Some(spell_slot) = input_state.spell.take() else {
            continue;
        };

        let Some(requested_spell_id) = player_registry.spell_at(client_id, spell_slot) else {
            push_cast_error(targeted_events, client_id, SpellUseError::SpellNotOwned);
            continue;
        };
        let Some(spell) = spell_registry.get_spell(requested_spell_id) else {
            tracing::error!("Le spell {requested_spell_id:?} n'est pas dans le registre");
            continue;
        };
        let cast_cost = spell.cast_cost;
        let used_spell_id =
            match player_registry.try_begin_spell_cast(client_id, spell_slot, cast_cost) {
                Ok(spell_id) => spell_id,
                Err(error) => {
                    push_cast_error(targeted_events, client_id, error);
                    continue;
                }
            };

        command.add_component(
            *entity,
            PendingSpellCast {
                aim_dir: input_state.aim_dir,
                spell_id: used_spell_id,
                slot: spell_slot,
            },
        );
        targeted_events.push(TargetedGameEvent {
            client_id,
            event: GameEvent {
                kind: utils::protocol::GameEventKind::SpellUsed { slot: spell_slot },
            },
        });
        if let Some(cooldowns) = player_registry.cooldowns(client_id) {
            targeted_events.push(TargetedGameEvent {
                client_id,
                event: GameEvent {
                    kind: utils::protocol::GameEventKind::SpellCooldownsUpdate { cooldowns },
                },
            });
        }
    }
}

fn push_cast_error(
    events: &mut crate::utils::Queue<TargetedGameEvent>,
    client_id: u64,
    error: SpellUseError,
) {
    let reason = match error {
        SpellUseError::PlayerNotFound | SpellUseError::SpellNotOwned => {
            utils::protocol::SpellCastErrorKind::SpellNotOwned
        }
        SpellUseError::CooldownActive => utils::protocol::SpellCastErrorKind::CooldownNotRefresh,
        SpellUseError::NotEnoughGold => utils::protocol::SpellCastErrorKind::NotEnoughtGold,
        SpellUseError::NoMoreCharges => utils::protocol::SpellCastErrorKind::NoMoreCharges,
    };
    events.push(TargetedGameEvent {
        client_id,
        event: GameEvent {
            kind: utils::protocol::GameEventKind::SpellCastError { reason },
        },
    });
}

#[system(for_each)]
#[filter(legion::component::<PendingSpellCast>())]
pub fn spell_cast_resolver(
    pending_cast: &PendingSpellCast,
    caster_entity: &legion::Entity,
    caster_pos: &Position,
    command: &mut CommandBuffer,
    #[resource] spell_registry: &SpellRegister,
) {
    let Some(spell) = spell_registry.get_spell(pending_cast.spell_id) else {
        tracing::error!(
            "Sort {:?} absent du registre pendant sa résolution",
            pending_cast.spell_id
        );
        command.remove_component::<PendingSpellCast>(*caster_entity);
        return;
    };

    match spell.targeting.kind {
        SpellTargetingKind::Directional => {
            let dir = pending_cast.aim_dir;
            let half_size = spell.targeting.projectile_radius;
            let speed = spell.targeting.speed.max(f32::EPSILON);
            let lifetime = (spell.targeting.range / speed).max(0.0);
            let projectile = command.push((
                EntityId(crate::app::next_id()),
                Position {
                    x: caster_pos.x,
                    y: caster_pos.y,
                },
                Velocity {
                    dx: dir[0] as f64 * speed as f64,
                    dy: dir[1] as f64 * speed as f64,
                },
                Geometry {
                    half_length: half_size,
                    half_width: half_size,
                    dir,
                },
                Projectile,
                Active(true),
                LifeTime(std::time::Duration::from_secs_f32(lifetime)),
            ));
            command.add_component(
                projectile,
                SpellEffects {
                    effects: spell.effects.clone(),
                    aoe: spell.targeting.aoe,
                },
            );
            command.add_component(projectile, TeamFilter { is_player: true });
            command.add_component(projectile, Owner(*caster_entity));
        }
        SpellTargetingKind::SingleTarget => {
            let dir = pending_cast.aim_dir;
            let center_x = caster_pos.x as f32 + dir[0] * spell.targeting.range;
            let center_y = caster_pos.y as f32 + dir[1] * spell.targeting.range;
            // Applique l'AOE immédiatement à la position visée
            command.push((
                PendingAoe {
                    origin: [center_x, center_y],
                    aim_dir: dir,
                    aoe: spell.targeting.aoe,
                    effects: spell.effects.clone(),
                    owner: *caster_entity,
                    caster_is_player: true,
                },
                Active(true),
            ));
        }

        SpellTargetingKind::OnSelf => {
            command.add_component(
                *caster_entity,
                PendingEffect {
                    effects: spell.effects.clone(),
                },
            );
        }
    }
    command.remove_component::<PendingSpellCast>(*caster_entity);
}

#[system]
#[read_component(Collider)]
#[read_component(Position)]
#[read_component(Health)]
#[read_component(PendingAoe)]
#[read_component(Player)]
pub fn apply_aoe(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    query_aoe: &mut legion::Query<(legion::Entity, &PendingAoe)>,
    #[resource] grid: &crate::navigation::SpatialGrid,
    #[resource] buff_manager: &mut utils::buffer::BufferManager,
    #[resource] damage_queue: &mut crate::utils::Queue<DamageEvent>,
    #[resource] active_burns: &mut ActiveBurns,
    #[resource] active_slows: &mut ActiveSlows,
    #[resource] support_effects: &mut crate::utils::Queue<SpellSupportEvent>,
) {
    let (victims_id, candidates_id) = (
        buff_manager.acquire_id::<Vec<(legion::Entity, Collider, Position)>>(),
        buff_manager.acquire_id::<Vec<usize>>(),
    );

    {
        let victims = buff_manager
            .get_mut::<Vec<(legion::Entity, Collider, Position)>>(victims_id)
            .unwrap();
        victims.extend(
            <(legion::Entity, &Collider, &Position)>::query()
                .filter(legion::component::<Health>())
                .iter(world)
                .map(|(e, c, p)| (*e, *c, *p)),
        );
    }

    for (aoe_entity, pending) in query_aoe.iter(world) {
        let hits: Vec<legion::Entity> = match &pending.aoe {
            None => {
                // Un ciblage SingleTarget sans forme d'AOE sélectionne l'entité
                // ennemie qui contient précisément le point visé.
                let point = Position {
                    x: pending.origin[0] as f64,
                    y: pending.origin[1] as f64,
                };
                let mut candidates = vec![];
                grid.query(&point, &Collider { w: 0.0, h: 0.0 }, &mut candidates);
                candidates.sort_unstable();
                candidates.dedup();

                let victims = buff_manager
                    .get::<Vec<(legion::Entity, Collider, Position)>>(victims_id)
                    .unwrap();
                candidates
                    .iter()
                    .find_map(|&idx| {
                        let (entity, collider, pos) = &victims[idx];
                        if *entity == pending.owner
                            || same_team(world, *entity, pending.caster_is_player)
                        {
                            return None;
                        }

                        let contains_point = point.x >= pos.x
                            && point.x <= pos.x + collider.w
                            && point.y >= pos.y
                            && point.y <= pos.y + collider.h;
                        contains_point.then_some(*entity)
                    })
                    .into_iter()
                    .collect()
            }
            Some(AoeSpellShape::Circle { offset, radius }) => {
                let cx = pending.origin[0] + offset.x;
                let cy = pending.origin[1] + offset.y;
                let r = *radius as f64;

                let broadphase_pos = Position {
                    x: cx as f64 - r,
                    y: cy as f64 - r,
                };
                let broadphase_col = Collider {
                    w: r * 2.0,
                    h: r * 2.0,
                };

                let mut candidates = vec![];
                grid.query(&broadphase_pos, &broadphase_col, &mut candidates);
                candidates.sort_unstable();
                candidates.dedup();

                let victims = buff_manager
                    .get::<Vec<(legion::Entity, Collider, Position)>>(victims_id)
                    .unwrap();
                candidates
                    .iter()
                    .filter_map(|&idx| {
                        let (entity, _, pos) = &victims[idx];
                        if *entity == pending.owner
                            || same_team(world, *entity, pending.caster_is_player)
                        {
                            return None;
                        }
                        let dx = pos.x - cx as f64;
                        let dy = pos.y - cy as f64;
                        if dx * dx + dy * dy <= r * r {
                            Some(*entity)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Some(AoeSpellShape::Box {
                offset,
                size,
                rotation,
            }) => {
                let cx = pending.origin[0] + offset.x;
                let cy = pending.origin[1] + offset.y;

                let aoe_pos = Position {
                    x: cx as f64,
                    y: cy as f64,
                };
                let aoe_geom = Geometry {
                    half_length: size.x / 2.0,
                    half_width: size.y / 2.0,
                    dir: [rotation.cos(), rotation.sin()],
                };

                let broadphase_w = (aoe_geom.half_width + aoe_geom.half_length) as f64;
                let broadphase_pos = Position {
                    x: cx as f64 - broadphase_w,
                    y: cy as f64 - broadphase_w,
                };
                let broadphase_col = Collider {
                    w: broadphase_w * 2.0,
                    h: broadphase_w * 2.0,
                };

                let mut candidates = vec![];
                grid.query(&broadphase_pos, &broadphase_col, &mut candidates);
                candidates.sort_unstable();
                candidates.dedup();

                let victims = buff_manager
                    .get::<Vec<(legion::Entity, Collider, Position)>>(victims_id)
                    .unwrap();
                candidates
                    .iter()
                    .filter_map(|&idx| {
                        let (entity, col, pos) = &victims[idx];
                        if *entity == pending.owner
                            || same_team(world, *entity, pending.caster_is_player)
                        {
                            return None;
                        }
                        if crate::utils::obb_vs_aabb(&aoe_pos, &aoe_geom, pos, col) {
                            Some(*entity)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Some(AoeSpellShape::Cone {
                offset,
                direction,
                angle,
                range,
            }) => {
                let cx = pending.origin[0] + offset.x;
                let cy = pending.origin[1] + offset.y;
                let r = *range as f64;
                let half_angle = (angle / 2.0).to_radians();

                let broadphase_pos = Position {
                    x: cx as f64 - r,
                    y: cy as f64 - r,
                };
                let broadphase_col = Collider {
                    w: r * 2.0,
                    h: r * 2.0,
                };

                let mut candidates = vec![];
                grid.query(&broadphase_pos, &broadphase_col, &mut candidates);
                candidates.sort_unstable();
                candidates.dedup();

                let victims = buff_manager
                    .get::<Vec<(legion::Entity, Collider, Position)>>(victims_id)
                    .unwrap();
                candidates
                    .iter()
                    .filter_map(|&idx| {
                        let (entity, _, pos) = &victims[idx];
                        if *entity == pending.owner
                            || same_team(world, *entity, pending.caster_is_player)
                        {
                            return None;
                        }
                        let dx = pos.x - cx as f64;
                        let dy = pos.y - cy as f64;
                        let dist_sq = dx * dx + dy * dy;

                        // Test distance
                        if dist_sq > r * r {
                            return None;
                        }

                        // Test angle — produit scalaire entre direction du cône et direction vers la cible
                        let dist = dist_sq.sqrt();
                        if dist < 0.001 {
                            return Some(*entity); // cible au centre du cône
                        }
                        let to_target_x = dx / dist;
                        let to_target_y = dy / dist;
                        let dot =
                            to_target_x * direction.x as f64 + to_target_y * direction.y as f64;
                        let cos_half_angle = (half_angle as f64).cos();

                        if dot >= cos_half_angle {
                            Some(*entity)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        };

        for target in hits {
            let entry = world.entry_ref(target).unwrap();
            let target_pos = entry.get_component::<Position>().unwrap();
            apply_effects(
                &pending.effects,
                target,
                pending.origin,
                [target_pos.x as f32, target_pos.y as f32],
                command,
                damage_queue,
                active_burns,
                active_slows,
                support_effects,
            );
        }

        command.remove(*aoe_entity);
    }

    buff_manager.release(victims_id);
    buff_manager.release(candidates_id);
}

fn same_team(world: &SubWorld, target: legion::Entity, caster_is_player: bool) -> bool {
    let target_is_player = world
        .entry_ref(target)
        .map(|entry| entry.get_component::<Player>().is_ok())
        .unwrap_or(false);
    target_is_player == caster_is_player
}

// TODO: Déplacer ça dans un endroit plus approprier
pub struct ActiveBurn {
    pub target: legion::Entity,
    pub remaining: f32,
    pub tick_interval: f32,
    pub tick_accumulator: f32,
    pub damage_per_tick: u32,
}

#[derive(Default)]
pub struct ActiveBurns {
    pub data: Vec<ActiveBurn>,
}

pub struct ActiveSlow {
    pub target: legion::Entity,
    pub remaining: f32,
    pub speed_multiplier: f32,
}
#[derive(Default)]
pub struct ActiveSlows {
    pub data: Vec<ActiveSlow>,
}

pub enum SpellSupportEvent {
    Heal {
        target: legion::Entity,
        amount: u32,
    },
    Blind {
        target: legion::Entity,
        duration: f32,
    },
}

#[system]
#[write_component(Health)]
pub fn apply_support_effects(
    world: &mut SubWorld,
    #[resource] effects: &mut crate::utils::Queue<SpellSupportEvent>,
    #[resource] players: &PlayerRegistry,
    #[resource] targeted_events: &mut crate::utils::Queue<TargetedGameEvent>,
) {
    for effect in effects.data.drain(..) {
        match effect {
            SpellSupportEvent::Heal { target, amount } => {
                if let Ok(mut entry) = world.entry_mut(target) {
                    if let Ok(health) = entry.get_component_mut::<Health>() {
                        // Le soin ne remplace pas le parcours de respawn.
                        if health.state == HealthState::Alive && health.hp > 0 {
                            health.hp = health.hp.saturating_add(amount).min(health.max_hp);
                        }
                    }
                }
            }
            SpellSupportEvent::Blind { target, duration } => {
                if !duration.is_finite() || duration <= 0.0 {
                    continue;
                }
                if let Some(client_id) = players
                    .iter_clients()
                    .find(|client_id| players.get_entity(*client_id) == Some(target))
                {
                    targeted_events.push(TargetedGameEvent {
                        client_id,
                        event: GameEvent {
                            kind: utils::protocol::GameEventKind::PlayerBlind { duration },
                        },
                    });
                }
            }
        }
    }
}

pub fn apply_effects(
    effects: &[SpellEffectKind],
    target: legion::Entity,
    origin: [f32; 2],
    target_pos: [f32; 2],
    command: &mut CommandBuffer,
    damage_queue: &mut crate::utils::Queue<crate::replication::DamageEvent>,
    active_burns: &mut ActiveBurns,
    active_slows: &mut ActiveSlows,
    support_effects: &mut crate::utils::Queue<SpellSupportEvent>,
) {
    for effect in effects {
        match effect {
            SpellEffectKind::Damage { amount, .. } => {
                damage_queue.data.push(crate::replication::DamageEvent {
                    target,
                    amount: *amount as u32,
                });
            }
            SpellEffectKind::Knockback { force } => {
                let dx = target_pos[0] - origin[0];
                let dy = target_pos[1] - origin[1];
                let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                command.add_component(
                    target,
                    Knockback {
                        dx: (dx / dist) * force,
                        dy: (dy / dist) * force,
                        duration: 0.12,
                    },
                );
            }
            SpellEffectKind::ApplyStatus { status, duration } => match status {
                AppliedStatus::Burn {
                    tick_interval,
                    damage_per_tick,
                } => {
                    active_burns.data.push(ActiveBurn {
                        target,
                        remaining: *duration,
                        tick_interval: *tick_interval,
                        tick_accumulator: 0.0,
                        damage_per_tick: *damage_per_tick as u32,
                    });
                }
                AppliedStatus::Blind => support_effects.push(SpellSupportEvent::Blind {
                    target,
                    duration: *duration,
                }),
                AppliedStatus::Slowed { speed_multiplier } => {
                    active_slows.data.push(ActiveSlow {
                        target,
                        remaining: *duration,
                        speed_multiplier: *speed_multiplier,
                    });
                }
            },
            SpellEffectKind::Heal { amount } => support_effects.push(SpellSupportEvent::Heal {
                target,
                amount: *amount,
            }),
        }
    }
}

#[system]
pub fn update_spell_cooldowns(
    #[resource] player_registry: &mut PlayerRegistry,
    #[resource] dt: &std::time::Duration,
    #[resource] targeted_events: &mut crate::utils::Queue<TargetedGameEvent>,
) {
    for (client_id, cooldowns) in player_registry.update_spell_cooldowns(dt.as_secs_f32()) {
        targeted_events.push(TargetedGameEvent {
            client_id,
            event: GameEvent {
                kind: utils::protocol::GameEventKind::SpellCooldownsUpdate { cooldowns },
            },
        });
    }
}

#[system(for_each)]
#[filter(legion::component::<PendingEffect>())]
pub fn apply_effect(
    entity: &legion::Entity,
    pending: &PendingEffect,
    pos: &Position,
    command: &mut CommandBuffer,
    #[resource] damage_queue: &mut crate::utils::Queue<DamageEvent>,
    #[resource] active_burns: &mut ActiveBurns,
    #[resource] active_slows: &mut ActiveSlows,
    #[resource] support_effects: &mut crate::utils::Queue<SpellSupportEvent>,
) {
    apply_effects(
        &pending.effects,
        *entity,
        [pos.x as f32, pos.y as f32],
        [pos.x as f32, pos.y as f32], // origin == target pour OnSelf
        command,
        damage_queue,
        active_burns,
        active_slows,
        support_effects,
    );
    command.remove_component::<PendingEffect>(*entity);
}

#[system(for_each)]
pub fn update_active_slows(
    entity: &Entity,
    velocity: &mut Velocity,
    #[resource] active_slows: &mut ActiveSlows,
    #[resource] dt: &std::time::Duration,
) {
    let elapsed = dt.as_secs_f32();
    for slow in &mut active_slows.data {
        if slow.target == *entity {
            let active_time = elapsed.min(slow.remaining);
            slow.remaining -= elapsed;

            if active_time > 0.0 {
                velocity.dx *= slow.speed_multiplier as f64;
                velocity.dy *= slow.speed_multiplier as f64;
            }
        }
    }
    active_slows.data.retain(|slow| slow.remaining > 0.0);
}

#[system]
pub fn update_active_burns(
    #[resource] burns: &mut ActiveBurns,
    #[resource] dt: &std::time::Duration,
    #[resource] damage_queue: &mut crate::utils::Queue<DamageEvent>,
) {
    let elapsed = dt.as_secs_f32();

    for burn in &mut burns.data {
        let active_time = elapsed.min(burn.remaining);

        burn.remaining -= elapsed;
        burn.tick_accumulator += active_time;

        while burn.tick_accumulator >= burn.tick_interval {
            damage_queue.push(DamageEvent {
                target: burn.target,
                amount: burn.damage_per_tick,
            });

            burn.tick_accumulator -= burn.tick_interval;
        }
    }

    burns.data.retain(|burn| burn.remaining > 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::systems::health::apply_damage_system;
    use legion::{EntityStore, IntoQuery, Resources, Schedule, World};

    #[test]
    fn heal_is_capped_and_does_not_revive_and_blind_targets_only_its_player() {
        let mut world = World::default();
        let player = world.push((Health {
            hp: 80,
            max_hp: 100,
            state: HealthState::Alive,
        },));
        let dead = world.push((Health {
            hp: 0,
            max_hp: 100,
            state: HealthState::Dead,
        },));
        let enemy = world.push(());
        let mut players = PlayerRegistry::with_capacity(2);
        players.add(42);
        players.link_entity(42, player, 1001);
        players.add(99);
        players.link_entity(99, dead, 1002);
        let mut resources = Resources::default();
        resources.insert(players);
        resources.insert(crate::utils::Queue::<TargetedGameEvent> { data: vec![] });
        resources.insert(crate::utils::Queue::<SpellSupportEvent> {
            data: vec![
                SpellSupportEvent::Heal {
                    target: player,
                    amount: u32::MAX,
                },
                SpellSupportEvent::Heal {
                    target: dead,
                    amount: 50,
                },
                SpellSupportEvent::Blind {
                    target: player,
                    duration: 4.0,
                },
                SpellSupportEvent::Blind {
                    target: enemy,
                    duration: 4.0,
                },
            ],
        });
        let mut schedule = Schedule::builder()
            .add_system(apply_support_effects_system())
            .build();
        schedule.execute(&mut world, &mut resources);
        schedule.execute(&mut world, &mut resources);
        assert_eq!(
            world
                .entry_ref(player)
                .unwrap()
                .get_component::<Health>()
                .unwrap()
                .hp,
            100
        );
        assert_eq!(
            world
                .entry_ref(dead)
                .unwrap()
                .get_component::<Health>()
                .unwrap()
                .hp,
            0
        );
        let events = resources
            .get::<crate::utils::Queue<TargetedGameEvent>>()
            .unwrap();
        assert_eq!(events.data.len(), 1);
        assert_eq!(events.data[0].client_id, 42);
        assert!(
            matches!(events.data[0].event.kind, utils::protocol::GameEventKind::PlayerBlind { duration } if duration == 4.0)
        );
    }

    #[test]
    fn pending_cast_is_consumed_and_creates_one_complete_projectile() {
        let config_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/config/spell.json");
        let registry = SpellRegister::init(config_path.to_str().unwrap()).unwrap();
        let spell_id = *registry.resolve_string("fireball").unwrap();
        let mut world = World::default();
        let caster = world.push((
            Position { x: 10.0, y: 20.0 },
            PendingSpellCast {
                aim_dir: [1.0, 0.0],
                spell_id,
                slot: utils::protocol::SpellSlot::First,
            },
        ));
        let mut resources = Resources::default();
        resources.insert(registry);
        let mut schedule = Schedule::builder()
            .add_system(spell_cast_resolver_system())
            .build();

        schedule.execute(&mut world, &mut resources);
        schedule.execute(&mut world, &mut resources);

        assert!(
            world
                .entry_ref(caster)
                .unwrap()
                .get_component::<PendingSpellCast>()
                .is_err()
        );
        assert!(
            world
                .entry_ref(caster)
                .unwrap()
                .get_component::<LifeTime>()
                .is_err()
        );

        let mut query = <(&Projectile, &LifeTime, &Active, &SpellEffects, &Owner)>::query();
        let projectiles: Vec<_> = query.iter(&world).collect();
        assert_eq!(projectiles.len(), 1);
        assert_eq!((projectiles[0].4).0, caster);
    }

    #[test]
    fn single_target_without_aoe_applies_effects_to_entity_under_target_point() {
        let mut world = World::default();
        let caster = world.push((Player,));
        let target_pos = Position { x: 100.0, y: 100.0 };
        let target_collider = Collider { w: 40.0, h: 40.0 };
        let target = world.push((
            IA,
            target_pos,
            target_collider,
            Health {
                hp: 100,
                max_hp: 100,
                state: HealthState::Alive,
            },
            Active(true),
        ));
        world.push((
            PendingAoe {
                origin: [110.0, 110.0],
                aim_dir: [1.0, 0.0],
                aoe: None,
                effects: vec![SpellEffectKind::Damage {
                    amount: 10.0,
                    element: utils::spell_types::Element::Fire,
                }],
                owner: caster,
                caster_is_player: true,
            },
            Active(true),
        ));

        let mut grid = crate::navigation::SpatialGrid::new(64.0, 1_000.0, 1_000.0);
        grid.insert(0, &target_pos, &target_collider);
        grid.build();

        let mut resources = Resources::default();
        resources.insert(grid);
        resources.insert(utils::buffer::BufferManager::with_capacity(4));
        resources.insert(crate::utils::Queue::<DamageEvent> { data: vec![] });
        resources.insert(crate::utils::Queue::<GameEvent> { data: vec![] });
        resources.insert(ActiveBurns::default());
        resources.insert(ActiveSlows::default());
        resources.insert(crate::utils::Queue::<SpellSupportEvent> { data: vec![] });

        let mut schedule = Schedule::builder()
            .add_system(apply_aoe_system())
            .add_system(apply_damage_system())
            .build();
        schedule.execute(&mut world, &mut resources);

        let hp = world
            .entry_ref(target)
            .unwrap()
            .get_component::<Health>()
            .unwrap()
            .hp;
        assert_eq!(hp, 90);
    }
}

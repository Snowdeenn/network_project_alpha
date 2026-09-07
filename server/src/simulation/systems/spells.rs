use crate::{
    replication::{DamageEvent, TargetedGameEvent},
    session::{PlayerRegistry, SpellUseError},
    simulation::resources::components::*,
    simulation::resources::spells::SpellRegister,
};
use legion::{EntityStore, query::IntoQuery, system, systems::CommandBuffer, world::SubWorld};
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
pub fn apply_aoe(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    query_aoe: &mut legion::Query<(legion::Entity, &PendingAoe)>,
    #[resource] grid: &crate::navigation::SpatialGrid,
    #[resource] buff_manager: &mut utils::buffer::BufferManager,
    #[resource] damage_queue: &mut crate::utils::Queue<DamageEvent>,
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
        let hits = match &pending.aoe {
            None => {
                // Pas d'AOE — effet ponctuel à l'origine, aucune entité cherchée ici
                // Le dégât a déjà été appliqué sur la cible directe dans check_collide_attackbox
                vec![]
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

pub fn apply_effects(
    effects: &[SpellEffectKind],
    target: legion::Entity,
    origin: [f32; 2],
    target_pos: [f32; 2],
    command: &mut CommandBuffer,
    damage_queue: &mut crate::utils::Queue<crate::replication::DamageEvent>,
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
            SpellEffectKind::ApplyStatus { .. } => {
                // à implémenter
            }
            SpellEffectKind::Heal { .. } => {
                // à implémenter
            }
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
) {
    apply_effects(
        &pending.effects,
        *entity,
        [pos.x as f32, pos.y as f32],
        [pos.x as f32, pos.y as f32], // origin == target pour OnSelf
        command,
        damage_queue,
    );
    command.remove_component::<PendingEffect>(*entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use legion::{EntityStore, IntoQuery, Resources, Schedule, World};

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
}

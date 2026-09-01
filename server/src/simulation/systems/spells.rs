use utils::spell_types::*;
use crate::{
    replication::DamageEvent,
    session::PlayerRegistry,
    simulation::resources::{
        components::*,
    },
    simulation::resources::spells::{SpellRegister},
};
use legion::{EntityStore, query::IntoQuery, system, systems::CommandBuffer, world::SubWorld};
use utils::protocol::GameEvent;

#[system]
#[read_component(InputState)]
#[read_component(EntityId)]
#[write_component(SpellCasted)]
pub fn listen_spell_cast(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    query: &mut legion::Query<(legion::Entity, &EntityId, &InputState)>,
    #[resource] player_registry: &mut PlayerRegistry,
    #[resource] spell_registry: &SpellRegister,
    #[resource] game_event_queue: &mut crate::utils::Queue<GameEvent>,
) {
    for (entity, entity_id, input_state) in query.iter(world) {
        let Some(client_id) = player_registry.entity_to_client(entity_id.0) else {
            tracing::warn!("L'entity n'est pas un joueur: EntityId => {entity_id:?}");
            continue;
        };
        let Some(player_spells) = player_registry.get_spells(client_id) else {
            tracing::error!("Echec lors de l'acquisition des spells du joueur : {client_id}");
            continue;
        };
        let Some(spell_slot) = input_state.spell else {
            continue;
        };

        let Some(used_spell_id) = player_spells[spell_slot as usize] else {
            game_event_queue.push(GameEvent {
                kind: utils::protocol::GameEventKind::SpellCastError {
                    reason: utils::protocol::SpellCastErrorKind::SpellNotOwned,
                },
            });
            continue;
        };
        let Some(spell) = spell_registry.get_spell(used_spell_id) else {
            tracing::error!("Le spell {used_spell_id:?} n'est pas dans le registre");
            continue;
        };
        let player_gold = player_registry.get_gold(client_id);

        if player_gold < spell.costs.gold {
            game_event_queue.push(GameEvent {
                kind: utils::protocol::GameEventKind::SpellCastError {
                    reason: utils::protocol::SpellCastErrorKind::NotEnoughtGold,
                },
            });
            continue;
        }
        let entity_entry = world.entry_ref(*entity).unwrap();
        let cooldowns = entity_entry.get_component::<SpellCooldowns>().unwrap();
        if cooldowns.slots[spell_slot as usize] > 0.0 {
            game_event_queue.push(GameEvent {
                kind: utils::protocol::GameEventKind::SpellCastError {
                    reason: utils::protocol::SpellCastErrorKind::CooldownNotRefresh,
                },
            });
            continue;
        }

        player_registry.sub_gold(client_id, spell.costs.gold);

        command.add_component(
            *entity,
            SpellCasted {
                aim_dir: input_state.aim_dir,
                cost: spell.costs,
                targeting: spell.targeting,
                effects: spell.effects.clone(),
            },
        );
        command.add_component(
            *entity,
            SpellCooldownStart {
                slot: spell_slot,
                duration: spell.costs.cooldown,
            },
        );
    }
}

#[system(for_each)]
#[filter(legion::component::<SpellCasted>())]
pub fn spell_cast_resolver(
    spell_casted: &SpellCasted,
    caster_entity: &legion::Entity,
    caster_pos: &Position,
    command: &mut CommandBuffer,
) {
    match spell_casted.targeting.kind {
        SpellTargetingKind::Directional => {
            let dir = spell_casted.aim_dir;
            let half_size = spell_casted.targeting.projectile_radius;
            command.push((
                EntityId(crate::app::next_id()),
                Position {
                    x: caster_pos.x,
                    y: caster_pos.y,
                },
                Velocity {
                    dx: dir[0] as f64 * spell_casted.targeting.speed as f64,
                    dy: dir[1] as f64 * spell_casted.targeting.speed as f64,
                },
                Geometry {
                    half_length: half_size,
                    half_width: half_size,
                    dir,
                },
                SpellEffects {
                    effects: spell_casted.effects.clone(),
                    aoe: spell_casted.targeting.aoe,
                },
                Projectile,
                TeamFilter { is_player: true },
                Owner(*caster_entity),
            ));
            command.add_component(*caster_entity, Active(true));
            command.add_component(
                *caster_entity,
                LifeTime(std::time::Duration::from_secs_f32(
                    spell_casted.targeting.range / spell_casted.targeting.speed,
                )),
            );
        }
        SpellTargetingKind::SingleTarget => {
            let dir = spell_casted.aim_dir;
            let center_x = caster_pos.x as f32 + dir[0] * spell_casted.targeting.range;
            let center_y = caster_pos.y as f32 + dir[1] * spell_casted.targeting.range;
            // Applique l'AOE immédiatement à la position visée
            command.push((
                PendingAoe {
                    origin: [center_x, center_y],
                    aim_dir: dir,
                    aoe: spell_casted.targeting.aoe,
                    effects: spell_casted.effects.clone(),
                },
                Active(true),
            ));
        }

        SpellTargetingKind::OnSelf => {
            command.add_component(
                *caster_entity,
                PendingEffect {
                    effects: spell_casted.effects.clone(),
                },
            );
        }
    }
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

#[system(for_each)]
pub fn update_spell_cooldowns(
    cooldowns: &mut SpellCooldowns,
    #[resource] dt: &std::time::Duration,
) {
    let dt = dt.as_secs_f32();
    for slot in cooldowns.slots.iter_mut() {
        *slot = (*slot - dt).max(0.0);
    }
}

#[system(for_each)]
pub fn start_spell_cooldown(
    entity: &legion::Entity,
    cooldown_start: &SpellCooldownStart,
    cooldowns: &mut SpellCooldowns,
    command: &mut CommandBuffer
) {
    let slot_index = cooldown_start.slot as usize;
    if slot_index < cooldowns.slots.len() {
        cooldowns.slots[slot_index] = cooldown_start.duration;
        command.remove_component::<SpellCooldownStart>(*entity);
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

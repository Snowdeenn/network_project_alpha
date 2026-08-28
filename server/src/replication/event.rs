use legion::world::Entity;
use utils::protocol::GameEvent;
use utils::protocol::GameEventKind;
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: u32,
}

#[derive(Debug)]
pub struct EnemyDied(pub Entity);

#[derive(Debug)]
pub struct CoinEvent {
    pub pos: [f32; 2],
}

pub fn process_game_event(
    net: &mut crate::net::GameNetServer,
    resources: &mut legion::Resources,
    world: &mut legion::world::SubWorld,
) {
    use crate::simulation::resources::components;
    use legion::EntityStore;

    let mut game_events = resources
        .get_mut::<crate::utils::Queue<utils::protocol::GameEvent>>()
        .expect("GameEventQueue pas dans les ressources");
    let mapping = resources
        .get::<crate::session::PlayerRegistry>()
        .expect("EntityToClient pas dans les ressources");

    for event in game_events.data.drain(..) {
        match event.kind {
            utils::protocol::GameEventKind::PlayerDied { entity_id } => {
                if let Some(client_id) = mapping.entity_to_client(entity_id) {
                    println!("Envoi de la mort au client concerné : {}", client_id);
                    net.send_event(client_id, &event);
                    net.send_event(
                        client_id,
                        &GameEvent {
                            kind: utils::protocol::GameEventKind::RespawnScheduled {
                                player_id: client_id, // Voir si on met le entity_id pour les autres joueurs
                                delay_secs: 10.0,
                            },
                        },
                    );

                    if let Some(entity) = resources
                        .get::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .get_entity(client_id)
                    {
                        if let Ok(mut entry) = world.entry_mut(entity) {
                            if let Ok(active) = entry.get_component_mut::<components::Active>() {
                                active.0 = false;
                            }
                            if let Ok(vel) = entry.get_component_mut::<components::Velocity>() {
                                vel.dx = 0.0;
                                vel.dy = 0.0;
                            }
                        }
                    }
                }
            }
            GameEventKind::RespawnAccept { client_id } => {
                net.send_event(
                    client_id,
                    &GameEvent {
                        kind: GameEventKind::RespawnAccept { client_id },
                    },
                );
            }
            _ => {
                net.broadcast_event(&event);
            }
        }
    }
}

pub fn process_incoming_game_event(
    net: &mut crate::net::GameNetServer,
    resources: &mut legion::Resources,
) {
    let Some(mut buff) = resources.get_mut::<utils::buffer::BufferManager>() else {
        tracing::error!("Resource BufferManager introuvable dans les resources legion");
        return;
    };
    let Some((event_id, events)) = buff.acquire::<Vec<(u64, utils::protocol::GameEvent)>>() else {
        tracing::error!(
            "Error lors de l'acquisition du buffer : Vec<(u64, GameEvent)> depuis le BufferManager"
        );
        return;
    };
    net.drain_game_event_into(events);
    for (_client_id, event) in events {
        match event.kind {
            utils::protocol::GameEventKind::RequestRespawn { client_id, option } => {
                let Some(mut registry) = resources.get_mut::<crate::session::PlayerRegistry>()
                else {
                    tracing::error!(
                        "Resource PlayerRegistry introuvable dans les resources legion"
                    );
                    continue;
                };
                let Some(mut game_event_queue) =
                    resources.get_mut::<crate::utils::Queue<GameEvent>>()
                else {
                    tracing::error!(
                        "Resource Queue<GameEvent> introuvable dans le resources legion"
                    );
                    continue;
                };

                match option {
                    utils::protocol::RespawnOption::UseGold => {
                        let player_gold_amount = registry.get_gold(client_id);

                        if player_gold_amount >= crate::config::RESPAWN_GOLD_AMOUNT {
                            registry.sub_gold(client_id, crate::config::RESPAWN_GOLD_AMOUNT);

                            game_event_queue.push(GameEvent {
                                kind: GameEventKind::RespawnPlayer { client_id },
                            });
                        } else {
                            game_event_queue.push(GameEvent {
                                kind: GameEventKind::RespawnError {
                                    reason: utils::protocol::RespawnErrorKind::NotEnoughtGold,
                                },
                            });
                        }
                    }
                    utils::protocol::RespawnOption::UseSharedLife => {
                        let Some(mut shared_lives) =
                            resources.get_mut::<crate::config::SharedLives>()
                        else {
                            tracing::error!(
                                "Resource SharedLife introuvable dans les resources legion"
                            );
                            continue;
                        };
                        shared_lives.remaining = shared_lives.remaining.saturating_sub(1);

                        if shared_lives.remaining != 0 {
                            // Broadcast vies restantes
                            game_event_queue.push(GameEvent {
                                kind: GameEventKind::SharedLivesUpdate {
                                    remaining: shared_lives.remaining,
                                    max: shared_lives.max,
                                },
                            });
                            game_event_queue.push(GameEvent {
                                kind: GameEventKind::RespawnPlayer { client_id },
                            });
                        } else {
                            game_event_queue.push(GameEvent {
                                kind: GameEventKind::RespawnError {
                                    reason:
                                        utils::protocol::RespawnErrorKind::NotEnoughtSharedLives,
                                },
                            });
                        }
                    }
                }
            }
            _ => {} // Message Server -> Client
        }
    }
    buff.release(event_id);
}

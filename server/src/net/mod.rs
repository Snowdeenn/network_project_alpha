pub use crate::net::server::GameNetServer;

pub mod server;

pub fn poll_event(
    net: &mut GameNetServer,
    resources: &mut legion::Resources,
    session: &mut crate::session::SessionState,
    world: &mut legion::World,
) {
    for event in net.drain_events() {
        match event {
            renet::ServerEvent::ClientConnected { client_id } => {
                tracing::info!("Client connecté: {client_id}");

                let game_cfg = resources.get::<utils::config::GameConfig>().unwrap();
                match session.add_slot(client_id, &game_cfg) {
                    Some(slot_index) => {
                        resources
                            .get_mut::<crate::session::PlayerRegistry>()
                            .unwrap()
                            .add(client_id);

                        net.send_lobby(
                            client_id,
                            &utils::protocol::LobbyMessage::SessionJoined {
                                code: session.code.clone(),
                                slot_index,
                            },
                        );

                        handle_reconnection(net, session, client_id, resources);
                    }
                    None => {
                        let msg = utils::protocol::LobbyMessage::SessionError {
                            reason: utils::protocol::SessionErrorKind::SessionFull,
                        };
                        net.send_lobby(client_id, &msg);
                    }
                }
            }
            renet::ServerEvent::ClientDisconnected { client_id, .. } => {
                println!("Client disconnected: {}", client_id);
                session.remove_slot(client_id);

                if let Some(entry) = resources
                    .get_mut::<crate::session::PlayerRegistry>()
                    .unwrap()
                    .remove(client_id)
                {
                    if let Some(entity) = entry.entity {
                        world.remove(entity);
                    }
                }

                net.broadcast_lobby(&utils::protocol::LobbyMessage::LobbyUpdate {
                    slots: session.to_slot_infos(),
                    phase: session.to_phase_info(),
                });
            }
        }
    }
}

fn handle_reconnection(
    net: &mut GameNetServer,
    session: &crate::session::SessionState,
    client_id: u64,
    resources: &legion::Resources,
) {
    if matches!(session.phase, crate::session::LobbyPhase::InGame) {
        net.send_lobby(
            client_id,
            &utils::protocol::LobbyMessage::GameStarting { countdown_secs: 0 },
        );

        if let Some(lives) = resources.get::<crate::config::SharedLives>() {
            net.send_event(
                client_id,
                &utils::protocol::GameEvent {
                    kind: utils::protocol::GameEventKind::SharedLivesUpdate {
                        remaining: lives.remaining,
                        max: lives.max,
                    },
                },
            );
        }
        if let Some(register) = resources.get::<crate::session::PlayerRegistry>() {
            if let Some(entry) = register.get_entry(client_id) {
                net.send_event(
                    client_id,
                    &utils::protocol::GameEvent {
                        kind: utils::protocol::GameEventKind::PlayerSpawn {
                            client_id,
                            entity_id: entry.entity_id.unwrap(),
                        },
                    },
                );
            }
        }
    } else {
        net.broadcast_lobby(&utils::protocol::LobbyMessage::LobbyUpdate {
            slots: session.to_slot_infos(),
            phase: session.to_phase_info(),
        });
    }
}

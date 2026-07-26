use crate::core::queue::Queue;
use crate::core::config::SharedLives;
use crate::net::server::GameNetServer;
use crate::app::next_id;
use crate::core::player_registry::PlayerRegistry;
use crate::core::session::{LobbyPhase, SessionState};
use crate::core::simulation::components::Position;
use crate::core::simulation::systems::spawn::spawn_player;
use crate::core::simulation::wave::{WaveManager, WaveState};
use legion::Resources;
use legion::World;
use shared::config::{ClassRegistery, GameConfig};
use shared::protocol::{GameEvent, GameEventKind, LobbyMessage, SessionErrorKind};
use std::time::Duration;

pub fn handle_lobby_message(
    client_id: u64,
    msg: LobbyMessage,
    session: &mut SessionState,
    net: &mut GameNetServer,
    world: &mut World,
    resources: &mut Resources,
) {
    match msg {
        LobbyMessage::ClassSelected { class } => {
            println!("ClassSelected reçu: {:?} de {}", class, client_id);
            session.set_class(client_id, class);
            broadcast_lobby_update(session, net);
        }

        LobbyMessage::ToggleReady => {
            println!("ToggleReady de {}", client_id);
            println!(
                "Slot class avant toggle : {:?}",
                session
                    .slots
                    .iter()
                    .flatten()
                    .find(|s| s.client_id == client_id)
                    .map(|s| s.class)
            );
            session.toggle_ready(client_id);
            println!("All ready : {}", session.all_ready());
            if session.all_ready() {
                session.phase = LobbyPhase::Starting {
                    countdown: Duration::from_secs(3),
                };
                let msg = LobbyMessage::GameStarting { countdown_secs: 3 };
                net.broadcast_lobby(&msg);
                // Spawn tous les joueurs
                start_game(session, world, resources);
            } else {
                broadcast_lobby_update(session, net);
            }
        }

        LobbyMessage::LeaveSession => {
            session.remove_slot(client_id);
            broadcast_lobby_update(session, net);
        }

        LobbyMessage::RequestJoinSession { code } => {
            if code != session.code {
                net.send_lobby(
                    client_id,
                    &LobbyMessage::SessionError {
                        reason: SessionErrorKind::InvalidCode,
                    },
                );
                return;
            }
            // Déjà géré dans ClientConnected pour la connexion initiale
            // Ce message sert si on implémente le flow "entrer un code manuellement"
            broadcast_lobby_update(session, net);
        }

        _ => {} // Messages serveur → client, ignorés si reçus
    }
}

fn start_game(session: &mut SessionState, world: &mut World, resources: &mut Resources) {
    let registry = resources.get::<ClassRegistery>().unwrap();
    let game_cfg = resources.get::<GameConfig>().unwrap();

    let spawn_points = &game_cfg.spawn_points;

    for (i, slot) in session.slots.iter().flatten().enumerate() {
        let class = slot.class.unwrap(); // garanti par all_ready()
        let pos = spawn_points
            .get(i)
            .map(|p| Position {
                x: p.x as f64,
                y: p.y as f64,
            })
            .unwrap_or(Position { x: 960.0, y: 540.0 });

        let player_game_id = next_id();
        let entity = spawn_player(world, player_game_id, &registry, class, pos);

        resources.get_mut::<PlayerRegistry>().unwrap().link_entity(
            slot.client_id,
            entity,
            player_game_id,
        );
    }

    session.phase = LobbyPhase::InGame;

    // Débloquer le WaveManager
    if let Some(mut wm) = resources.get_mut::<WaveManager>() {
        wm.wave_state = WaveState::InProgress;
    }

    if let Some(lives) = resources.get::<SharedLives>() {
        if let Some(mut event_queue) = resources.get_mut::<Queue<GameEvent>>() {
            event_queue.data.push(GameEvent {
                kind: GameEventKind::SharedLivesUpdate {
                    remaining: lives.remaining,
                    max: lives.max,
                },
            });
        }
    }
}

fn broadcast_lobby_update(session: &SessionState, net: &mut GameNetServer) {
    let update = LobbyMessage::LobbyUpdate {
        slots: session.to_slot_infos(),
        phase: session.to_phase_info(),
    };
    net.broadcast_lobby(&update);
}

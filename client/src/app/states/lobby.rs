use utils::{
    config::PlayerClass,
    protocol::{LobbyMessage, LobbyPhaseInfo},
};

use crate::{
    app::input::Input,
    core::{
        client::GameNetClient,
        event::{AppScreen, ClientState, LobbyScreenState},
    },
    rendering::ScreenScale,
};

pub fn handle_lobby_message(msg: LobbyMessage, screen: &mut AppScreen, is_solo: &mut bool) {
    match msg {
        LobbyMessage::SessionJoined { code, slot_index } => {
            *screen = AppScreen::Lobby(LobbyScreenState {
                code,
                slot_index,
                slots: vec![None; 4],
                my_class: None,
                ready: false,
                phase: LobbyPhaseInfo::Waiting,
                is_solo: *is_solo,
            })
        }
        LobbyMessage::LobbyUpdate { slots, phase } => {
            if let AppScreen::Lobby(state) = screen {
                state.slots = slots;
                state.phase = phase;
            }
        }
        LobbyMessage::GameStarting { .. } => {
            *screen = AppScreen::InGame(ClientState::new());
        }
        LobbyMessage::SessionError { reason } => {
            // TODO: afficher l'erreur dans le MainMenu
            println!("Session error: {:?}", reason);
        }
        _ => {}
    }
}

pub fn handle_input(input_state: &Input, state: &mut LobbyScreenState, client: &mut GameNetClient) {
    let class_keys = [
        (winit::keyboard::KeyCode::KeyO, PlayerClass::Warrior),
        (winit::keyboard::KeyCode::Numpad2, PlayerClass::Assassin),
        (winit::keyboard::KeyCode::Numpad3, PlayerClass::Mage),
        (winit::keyboard::KeyCode::Numpad4, PlayerClass::Tank),
    ];

    for (key, class) in class_keys {
        if input_state.is_just_pressed(key) {
            tracing::info!("Class selected : {class:?}");
            state.my_class = Some(class);
            client.send_lobby_message(&LobbyMessage::ClassSelected { class });

            if state.is_solo && !state.ready {
                state.ready = true;
                tracing::info!("Auto Ready envoyé");
                client.send_lobby_message(&LobbyMessage::ToggleReady);
            }
        }
    }

    if !state.is_solo
        && input_state.is_just_pressed(winit::keyboard::KeyCode::Space)
        && state.my_class.is_some()
    {
        tracing::info!("Toggle Ready envoyé");
        state.ready = !state.ready;
        client.send_lobby_message(&LobbyMessage::ToggleReady);
    }
}

pub fn render(frame: &mut prism::Frame, state: &LobbyScreenState, s: &ScreenScale) {
    frame.push_hud(prism::DrawCommand::Text {
        content: format!("Code : {}", state.code),
        pos: [s.x(0.02) as f32, s.y(0.02) as f32],
        size: s.font(0.03) as f32,
        color: [
            (utils::colors::Color::GOLD.r as f32) / 255.0,
            (utils::colors::Color::GOLD.g as f32) / 255.0,
            (utils::colors::Color::GOLD.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });

    // Slots joueurs
    for (i, slot) in state.slots.iter().enumerate() {
        let x = s.x(0.1 + i as f32 * 0.22);
        let y = s.y(0.35);
        let w = s.w(0.18);
        let h = s.h(0.3);

        // Fond du slot
        let bg = if slot.is_some() {
            utils::colors::Color::DARKGRAY
        } else {
            utils::colors::Color::new(30, 30, 30, 255)
        };
        frame.push_hud(prism::DrawCommand::Shape {
            shape: prism::Shape::Quad {
                pos: [x as f32, y as f32],
                size: [w as f32, h as f32],
                rotation: 0.0,
                color: [
                    (bg.r as f32) / 255.0,
                    (bg.g as f32) / 255.0,
                    (bg.b as f32) / 255.0,
                    1.0,
                ],
                uv: None,
            },
            blend: prism::BlendMode::Opaque,
            layer: 0,
        });
        match slot {
            None => {
                frame.push_hud(prism::DrawCommand::Text {
                    content: "En attente ...".to_string(),
                    pos: [(x + s.x(0.02)) as f32, (y + s.y(0.12)) as f32],
                    size: s.font(0.02) as f32,
                    color: [
                        (utils::colors::Color::GRAY.r as f32) / 255.0,
                        (utils::colors::Color::GRAY.b as f32) / 255.0,
                        (utils::colors::Color::GRAY.g as f32) / 255.0,
                        1.0,
                    ],
                    layer: 0,
                });
            }
            Some(info) => {
                frame.push_hud(prism::DrawCommand::Text {
                    content: format!("Joueur {}", info.slot_index + 1),
                    pos: [(x + s.x(0.01)) as f32, (y + s.y(0.02)) as f32],
                    size: s.font(0.025) as f32,
                    color: [1.0, 1.0, 1.0, 1.0], // BLANC
                    layer: 0,
                });
                // Classe
                let class_text = if info.slot_index == state.slot_index {
                    match state.my_class {
                        None => "Aucune classe".to_string(),
                        Some(c) => format!("{:?}", c),
                    }
                } else {
                    match info.class {
                        None => "Aucune classe".to_string(),
                        Some(c) => format!("{:?}", c),
                    }
                };
                frame.push_hud(prism::DrawCommand::Text {
                    content: class_text,
                    pos: [(x + s.x(0.01)) as f32, (y + s.y(0.1)) as f32],
                    size: s.font(0.022) as f32,
                    color: [
                        (utils::colors::Color::SKYBLUE.r as f32) / 255.0,
                        (utils::colors::Color::SKYBLUE.g as f32) / 255.0,
                        (utils::colors::Color::SKYBLUE.b as f32) / 255.0,
                        1.0,
                    ],
                    layer: 0,
                });
                // Ready
                let (ready_text, ready_color) = if info.slot_index == state.slot_index {
                    if state.ready {
                        ("PRÊT ✓", utils::colors::Color::GREEN)
                    } else {
                        ("PAS PRÊT", utils::colors::Color::RED)
                    }
                } else {
                    if info.ready {
                        ("PRÊT ✓", utils::colors::Color::GREEN)
                    } else {
                        ("PAS PRÊT", utils::colors::Color::RED)
                    }
                };
                frame.push_hud(prism::DrawCommand::Text {
                    content: ready_text.to_string(),
                    pos: [(x + s.x(0.01)) as f32, (y + s.y(0.2)) as f32],
                    size: s.font(0.022) as f32,
                    color: [
                        (ready_color.r as f32) / 255.0,
                        (ready_color.g as f32) / 255.0,
                        (ready_color.b as f32) / 255.0,
                        1.0,
                    ],
                    layer: 0,
                });
                // Marquer le slot local
                if info.slot_index == state.slot_index {
                    let gold_color = [
                        (utils::colors::Color::GOLD.r as f32) / 255.0,
                        (utils::colors::Color::GOLD.g as f32) / 255.0,
                        (utils::colors::Color::GOLD.b as f32) / 255.0,
                        1.0,
                    ];
                    let mut mesh = prism::RawMesh::with_capacity(4, 6);
                    let i0 = mesh.push_vertex(prism::Vertex {
                        pos: [x as f32, y as f32],
                        uv: [0.0, 0.0],
                        color: gold_color,
                    });
                    let i1 = mesh.push_vertex(prism::Vertex {
                        pos: [(x + w) as f32, y as f32],
                        uv: [0.0, 0.0],
                        color: gold_color,
                    });
                    let i2 = mesh.push_vertex(prism::Vertex {
                        pos: [x as f32, (y + h) as f32],
                        uv: [0.0, 0.0],
                        color: gold_color,
                    });
                    let i3 = mesh.push_vertex(prism::Vertex {
                        pos: [(x + w) as f32, (y + h) as f32],
                        uv: [0.0, 0.0],
                        color: gold_color,
                    });
                    mesh.push_triangle(i0, i1, i2);
                    mesh.push_triangle(i1, i3, i2);

                    frame.push_hud(prism::DrawCommand::Mesh {
                        mesh,
                        blend: prism::BlendMode::Alpha,
                        layer: 0,
                    });
                }
            }
        }
    }

    // Instructions
    frame.push_hud(prism::DrawCommand::Text {
        content: "1/2/3/4 — Choisir une classe    ESPACE — Prêt".to_string(),
        pos: [s.x(0.25) as f32, s.y(0.75) as f32],
        size: s.font(0.025) as f32,
        color: [
            (utils::colors::Color::LIGHTGRAY.r as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.g as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });
    // Classe choisie localement
    if let Some(class) = state.my_class {
        frame.push_hud(prism::DrawCommand::Text {
            content: format!("Ta classe : {:?}", class),
            pos: [s.x(0.02) as f32, s.y(0.9) as f32],
            size: s.font(0.028) as f32,
            color: [
                (utils::colors::Color::GOLD.r as f32) / 255.0,
                (utils::colors::Color::GOLD.g as f32) / 255.0,
                (utils::colors::Color::GOLD.b as f32) / 255.0,
                1.0,
            ],
            layer: 0,
        });
    }
}

use raylib::prelude::*;
use shared::{
    config::PlayerClass,
    protocol::{LobbyMessage, LobbyPhaseInfo},
};

use crate::{
    event::{AppScreen, ClientState, LobbyScreenState},
    net::client::GameNetClient,
    renderer::ScreenScale,
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

pub fn handle_input(rl: &RaylibHandle, state: &mut LobbyScreenState, client: &mut GameNetClient) {
    let class_keys = [
        (KeyboardKey::KEY_ONE, PlayerClass::Warrior),
        (KeyboardKey::KEY_TWO, PlayerClass::Assassin),
        (KeyboardKey::KEY_THREE, PlayerClass::Mage),
        (KeyboardKey::KEY_FOUR, PlayerClass::Tank),
    ];

    for (key, class) in class_keys {
        if rl.is_key_pressed(key) {
            println!("Classe selected : {:?}", class);
            state.my_class = Some(class);
            client.send_lobby_message(&LobbyMessage::ClassSelected { class });

            if state.is_solo && !state.ready {
                state.ready = true;
                println!("Auto-ready solo envoyé");
                client.send_lobby_message(&LobbyMessage::ToggleReady);
            }
        }
    }

    if !state.is_solo && rl.is_key_pressed(KeyboardKey::KEY_SPACE) && state.my_class.is_some() {
        println!("Envoi ToggleReady");
        state.ready = !state.ready;
        client.send_lobby_message(&LobbyMessage::ToggleReady);
    }
}

pub fn render(d: &mut RaylibDrawHandle, state: &LobbyScreenState, s: &ScreenScale) {
    d.clear_background(Color::BLACK);

    // Code de session
    d.draw_text(
        &format!("Code : {}", state.code),
        s.x(0.02),
        s.y(0.02),
        s.font(0.03),
        Color::GOLD,
    );

    // Slots joueurs
    for (i, slot) in state.slots.iter().enumerate() {
        let x = s.x(0.1 + i as f32 * 0.22);
        let y = s.y(0.35);
        let w = s.w(0.18);
        let h = s.h(0.3);

        // Fond du slot
        let bg = if slot.is_some() {
            Color::DARKGRAY
        } else {
            Color::new(30, 30, 30, 255)
        };
        d.draw_rectangle(x, y, w, h, bg);

        match slot {
            None => {
                d.draw_text(
                    "En attente...",
                    x + s.x(0.02),
                    y + s.y(0.12),
                    s.font(0.02),
                    Color::GRAY,
                );
            }
            Some(info) => {
                // Nom joueur
                d.draw_text(
                    &format!("Joueur {}", info.slot_index + 1),
                    x + s.x(0.01),
                    y + s.y(0.02),
                    s.font(0.025),
                    Color::WHITE,
                );

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

                d.draw_text(
                    &class_text,
                    x + s.x(0.01),
                    y + s.y(0.1),
                    s.font(0.022),
                    Color::SKYBLUE,
                );

                // Ready
                let (ready_text, ready_color) = if info.slot_index == state.slot_index {
                    if state.ready {
                        ("PRÊT ✓", Color::GREEN)
                    } else {
                        ("PAS PRÊT", Color::RED)
                    }
                } else {
                    if info.ready {
                        ("PRÊT ✓", Color::GREEN)
                    } else {
                        ("PAS PRÊT", Color::RED)
                    }
                };

                d.draw_text(
                    ready_text,
                    x + s.x(0.01),
                    y + s.y(0.2),
                    s.font(0.022),
                    ready_color,
                );

                // Marquer le slot local
                if info.slot_index == state.slot_index {
                    d.draw_rectangle_lines(x, y, w, h, Color::GOLD);
                }
            }
        }
    }

    // Instructions
    d.draw_text(
        "1/2/3/4 — Choisir une classe    ESPACE — Prêt",
        s.x(0.25),
        s.y(0.75),
        s.font(0.025),
        Color::LIGHTGRAY,
    );

    // Classe choisie localement
    if let Some(class) = state.my_class {
        d.draw_text(
            &format!("Ta classe : {:?}", class),
            s.x(0.02),
            s.y(0.9),
            s.font(0.028),
            Color::GOLD,
        );
    }
}

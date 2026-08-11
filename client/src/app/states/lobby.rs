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
};

#[derive(Debug, Clone, Copy)]
pub struct SlotId {
    pub root: nodus::NodeId,
    pub waiting_text: nodus::NodeId,
    pub player_name: nodus::NodeId,
    pub class_text: nodus::NodeId,
    pub ready_text: nodus::NodeId,
    pub gold_border: nodus::NodeId, // overlay doré pour le slot local
}

pub fn init_lobby(ui_ctx: &mut nodus::UiContext, register: &mut utils::ids::Register) {
    let root = ui_ctx.add_node(
        ui_ctx.root,
        nodus::LayoutProps::new(
            nodus::Anchor::TopLeft,
            nodus::UiVec2::pixels(0.0, 0.0),
            nodus::UiVec2::new(
                nodus::UiUnit::ParentPercent(1.0),
                nodus::UiUnit::ParentPercent(1.0),
            ),
        ),
        nodus::VisualProps {
            kind: nodus::VisualKind::Rect,
            color: utils::colors::Color::TRANSPARENT,
            visible: false, // caché par défaut
            opacity: 1.0,
        },
    );
    register.insert(crate::key::lobby::ROOT, root);

    let code_label = nodus::text_label! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(0.02, 0.02),
        size: nodus::UiVec2::screen(0.3, 0.05),
        content: "Code : ----",
        font_size: 24.0,
        color: utils::colors::Color::GOLD,
    };
    register.insert(crate::key::lobby::CODE_LABEL, code_label);

    for i in 0..4usize {
        let x = 0.1 + i as f32 * 0.22;

        let slot_root = ui_ctx.add_node(
            root,
            nodus::LayoutProps::new(
                nodus::Anchor::TopLeft,
                nodus::UiVec2::screen(x, 0.35),
                nodus::UiVec2::screen(0.18, 0.3),
            ),
            nodus::VisualProps {
                kind: nodus::VisualKind::Rect,
                color: utils::colors::Color::new(30, 30, 30, 255),
                visible: true,
                opacity: 1.0,
            },
        );

        let waiting_text = nodus::text_label! {
            ctx: ui_ctx,
            parent: slot_root,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(0.02, 0.12),
            size: nodus::UiVec2::screen(0.14, 0.03),
            content: "En attente ...",
            font_size: 16.0,
            color: utils::colors::Color::GRAY,
        };

        let player_name = nodus::text_label! {
            ctx: ui_ctx,
            parent: slot_root,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(0.01, 0.02),
            size: nodus::UiVec2::screen(0.16, 0.03),
            content: "",
            font_size: 20.0,
            color: utils::colors::Color::WHITE,
        };

        let class_text = nodus::text_label! {
            ctx: ui_ctx,
            parent: slot_root,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(0.01, 0.1),
            size: nodus::UiVec2::screen(0.16, 0.03),
            content: "",
            font_size: 18.0,
            color: utils::colors::Color::SKYBLUE,
        };

        let ready_text = nodus::text_label! {
            ctx: ui_ctx,
            parent: slot_root,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(0.01, 0.2),
            size: nodus::UiVec2::screen(0.16, 0.03),
            content: "",
            font_size: 18.0,
            color: utils::colors::Color::RED,
        };

        // Overlay doré pour le slot local — caché par défaut
        let gold_border = ui_ctx.add_node(
            slot_root,
            nodus::LayoutProps::new(
                nodus::Anchor::TopLeft,
                nodus::UiVec2::pixels(0.0, 0.0),
                nodus::UiVec2::new(
                    nodus::UiUnit::ParentPercent(1.0),
                    nodus::UiUnit::ParentPercent(1.0),
                ),
            ),
            nodus::VisualProps {
                kind: nodus::VisualKind::Rect,
                color: utils::colors::Color::GOLD,
                visible: false,
                opacity: 0.15,
            },
        );
        let slot_id = SlotId {
            root: slot_root,
            waiting_text,
            player_name,
            class_text,
            ready_text,
            gold_border,
        };
        register.insert(crate::key::lobby::SLOT_KEYS[i], slot_id);
    }

    let instructions = nodus::text_label! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(0.25, 0.75),
        size: nodus::UiVec2::screen(0.5, 0.04),
        content: "1/2/3/4 — Choisir une classe    ESPACE — Prêt",
        font_size: 20.0,
        color: utils::colors::Color::LIGHTGRAY,
    };
    register.insert(crate::key::lobby::INSTRUCTION, instructions);

    let class_label = nodus::text_label! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(0.02, 0.9),
        size: nodus::UiVec2::screen(0.3, 0.04),
        content: "",
        font_size: 22.0,
        color: utils::colors::Color::GOLD,
    };
    register.insert(crate::key::lobby::CLASS, class_label);
}

pub fn update(ui_ctx: &mut nodus::UiContext, ids: &utils::ids::Register, state: &LobbyScreenState) {
    let code_label = match ids.get::<nodus::NodeId>(crate::key::lobby::CODE_LABEL) {
        Some(id) => id,
        None => {
            tracing::warn!(
                "L'id {} est absent du register",
                crate::key::lobby::CODE_LABEL
            );
            return;
        }
    };
    // Code session
    ui_ctx.send_event(nodus::UIEvent::SetText {
        target: code_label,
        content: format!("Code : {}", state.code),
    });

    // Slots
    for (i, slot) in state.slots.iter().enumerate() {
        let slot_id = match ids.get::<SlotId>(crate::key::lobby::SLOT_KEYS[i]) {
            Some(id) => id,
            None => {
                tracing::warn!(
                    "L'id {} est absent du register",
                    crate::key::lobby::SLOT_KEYS[i]
                );
                return;
            }
        };
        let is_local = i == state.slot_index as usize;
        match slot {
            None => {
                // Fond vide
                ui_ctx.send_event(nodus::UIEvent::SetColor {
                    target: slot_id.root,
                    color: utils::colors::Color::new(30, 30, 30, 255),
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.waiting_text,
                    visible: true,
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.player_name,
                    visible: false,
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.class_text,
                    visible: false,
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.ready_text,
                    visible: false,
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.gold_border,
                    visible: false,
                });
            }
            Some(info) => {
                // Fond occupé
                ui_ctx.send_event(nodus::UIEvent::SetColor {
                    target: slot_id.root,
                    color: utils::colors::Color::DARKGRAY,
                });
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.waiting_text,
                    visible: false,
                });

                // Nom joueur
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.player_name,
                    visible: true,
                });
                ui_ctx.send_event(nodus::UIEvent::SetText {
                    target: slot_id.player_name,
                    content: format!("Joueur {}", info.slot_index + 1),
                });

                // Classe
                let class_text = if is_local {
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
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.class_text,
                    visible: true,
                });
                ui_ctx.send_event(nodus::UIEvent::SetText {
                    target: slot_id.class_text,
                    content: class_text,
                });

                // Ready
                let (ready_text, ready_color) = if is_local {
                    if state.ready {
                        ("PRÊT ✓", utils::colors::Color::GREEN)
                    } else {
                        ("PAS PRÊT", utils::colors::Color::RED)
                    }
                } else if info.ready {
                    ("PRÊT ✓", utils::colors::Color::GREEN)
                } else {
                    ("PAS PRÊT", utils::colors::Color::RED)
                };
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.ready_text,
                    visible: true,
                });
                ui_ctx.send_event(nodus::UIEvent::SetText {
                    target: slot_id.ready_text,
                    content: ready_text.to_string(),
                });
                ui_ctx.send_event(nodus::UIEvent::SetColor {
                    target: slot_id.ready_text,
                    color: ready_color,
                });

                // Overlay doré pour le slot local
                ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: slot_id.gold_border,
                    visible: is_local,
                });
            }
        }
    }

    // Classe locale
    let class_label = match ids.get::<nodus::NodeId>(crate::key::lobby::CLASS) {
            Some(id) => id,
            None => {
                tracing::warn!(
                    "L'id {} est absent du register",
                    crate::key::lobby::CLASS
                );
                return;
            }
        };
    match state.my_class {
        None => {
            ui_ctx.send_event(nodus::UIEvent::SetText {
                target: class_label,
                content: String::new(),
            });
        }
        Some(class) => {
            ui_ctx.send_event(nodus::UIEvent::SetText {
                target: class_label,
                content: format!("Ta classe : {:?}", class),
            });
        }
    }
}

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

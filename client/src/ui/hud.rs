use crate::app::resources::Resources;
use crate::app::states::in_game::{GuiContext, HudBuffers};
use crate::core::config::*;
use nodus::VisualProps;
use std::fmt::Write;
use utils::ids::MaterialId;
use utils::protocol::StateSnapshot;

pub fn init_hud(
    ui_ctx: &mut nodus::UiContext,
    hp_material_id: MaterialId,
    register: &mut utils::ids::Register,
) {
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
            kind: nodus::VisualKind::None, // Node qui contient tout le hud
            color: utils::colors::Color::TRANSPARENT,
            visible: false, // caché par défaut
            opacity: 1.0,
        },
    );
    register.insert(crate::key::hud::ROOT, root);
    let wave_label_id = nodus::text_label! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(HUD_PADDING_X, HUD_WAVE_Y),
        size: nodus::UiVec2::screen(0.2, 0.05),
        content: "Vague 0 | 0 ennemis",
        font_size: 24.0,
        color: utils::colors::Color::WHITE,
    };
    register.insert(crate::key::hud::WAVE_LABEL, wave_label_id);

    let (hp_bg_id, hp_fill_id) = nodus::progress_bar! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(HUD_PADDING_X, HUD_BAR_Y),
        size: nodus::UiVec2::screen(HUD_BAR_W, HUD_BAR_H),
        bg: utils::colors::Color::DARKGRAY,
        fill_color: utils::colors::Color::WHITE,
        material: hp_material_id,
        // ratio initial à 1.0 (barre pleine) encodé en f32 little-endian
        uniform_data: bytemuck::cast_slice(&[1.0f32]).to_vec(),
    };
    register.insert(crate::key::hud::HP_BG, hp_bg_id);
    register.insert(crate::key::hud::HP_FILL, hp_fill_id);

    let hp_text_id = nodus::text_label! {
        ctx: ui_ctx,
        parent: hp_bg_id,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::pixels(5.0, 2.0),
        size: nodus::UiVec2::new(nodus::UiUnit::ParentPercent(1.0), nodus::UiUnit::ParentPercent(1.0)),
        content: "100/100",
        font_size: 16.0,
        color: utils::colors::Color::WHITE,
    };
    register.insert(crate::key::hud::HP_TEXT, hp_text_id);

    let gold_label_id = nodus::text_label! {
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(HUD_PADDING_X, HUD_GOLD_Y),
        size: nodus::UiVec2::screen(0.1, 0.03),
        content: "Or : 0",
        font_size: 24.0,
        color: utils::colors::Color::GOLD,
    };
    register.insert(crate::key::hud::GOLD_LABEL, gold_label_id);

    let respawn_label_id = ui_ctx.add_node(
        root,
        nodus::LayoutProps::new(
            nodus::Anchor::BottomLeft,
            nodus::UiVec2::new(nodus::UiUnit::Pixels(540.0), nodus::UiUnit::Pixels(30.0)),
            nodus::UiVec2::pixels(50.0, 25.0),
        ),
        nodus::VisualProps {
            kind: nodus::VisualKind::Text {
                content: "".to_string(),
                font_size: 40.0,
            },
            color: utils::colors::Color::RED,
            visible: false,
            opacity: 1.0,
        },
    );
    register.insert(crate::key::hud::RESPAWN_LABEL, respawn_label_id);

    let respawn_shared_lives_button = ui_ctx.add_node(
        root,
        nodus::LayoutProps::new(
            nodus::Anchor::Center,
            nodus::UiVec2::new(nodus::UiUnit::Pixels(-300.0), nodus::UiUnit::Pixels(-114.0)),
            nodus::UiVec2::pixels(200.0, 50.0),
        ),
        VisualProps {
            kind: nodus::VisualKind::Rect,
            color: utils::colors::Color::BLACK,
            opacity: 1.0,
            visible: false,
        },
    );
    ui_ctx.set_interact(
        respawn_shared_lives_button,
        nodus::Interact {
            state: nodus::InteractState::Normal,
            style: nodus::ButtonStyle {
                normal: utils::colors::Color::BLACK,
                hover: utils::colors::Color::GRAY,
                pressed: utils::colors::Color::LIGHTGRAY,
            },
        },
    );
    register.insert(
        crate::key::hud::RESPAWN_SHARED_LIVES_BUTTON,
        respawn_shared_lives_button,
    );

    let respawn_shared_life_label = nodus::text_label!(
        ctx: ui_ctx,
        parent: respawn_shared_lives_button,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::new(
            nodus::UiUnit::ParentPercent(0.2),
            nodus::UiUnit::ParentPercent(0.3)
        ),
        size: nodus::UiVec2::new(
            nodus::UiUnit::ParentPercent(0.25),
            nodus::UiUnit::ParentPercent(0.3)
        ),
        content: "Shared Lives".to_string(),
        font_size: 15.0,
        color: utils::colors::Color::WHITE,
    );
    register.insert(
        crate::key::hud::RESPAWN_SHARED_LIVES_BUTTON_LABEL,
        respawn_shared_life_label,
    );

    let respawn_gold_button = ui_ctx.add_node(
        root,
        nodus::LayoutProps::new(
            nodus::Anchor::Center,
            nodus::UiVec2::new(nodus::UiUnit::Pixels(300.0), nodus::UiUnit::Pixels(-114.0)),
            nodus::UiVec2::pixels(200.0, 50.0),
        ),
        nodus::VisualProps {
            kind: nodus::VisualKind::Rect,
            color: utils::colors::Color::BLACK,
            visible: false,
            opacity: 1.0,
        },
    );
    ui_ctx.set_interact(
        respawn_gold_button,
        nodus::Interact {
            state: nodus::InteractState::Normal,
            style: nodus::ButtonStyle {
                normal: utils::colors::Color::BLACK,
                hover: utils::colors::Color::GRAY,
                pressed: utils::colors::Color::LIGHTGRAY,
            },
        },
    );
    register.insert(crate::key::hud::RESPAWN_GOLD_BUTTON, respawn_gold_button);

    let respawn_gold_button_label = nodus::text_label!(
        ctx: ui_ctx,
        parent: respawn_gold_button,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::new(
            nodus::UiUnit::ParentPercent(0.4),
            nodus::UiUnit::ParentPercent(0.3)
        ),
        size: nodus::UiVec2::new(
            nodus::UiUnit::ParentPercent(0.25),
            nodus::UiUnit::ParentPercent(0.3)
        ),
        content: "Gold".to_string(),
        font_size: 15.0,
        color: utils::colors::Color::WHITE,
    );
    register.insert(
        crate::key::hud::RESPAWN_GOLD_BUTTON_LABEL,
        respawn_gold_button_label,
    );

    let shared_lives_label = nodus::text_label!(
        ctx: ui_ctx,
        parent: root,
        anchor: nodus::Anchor::TopRight,
        offset: nodus::UiVec2::new(nodus::UiUnit::Pixels(-400.0), nodus::UiUnit::Pixels(-50.0)),
        size: nodus::UiVec2::new(nodus::UiUnit::Pixels(200.0), nodus::UiUnit::Pixels(30.0)),
        content: "Shared Lives: 0".to_string(),
        font_size: 24.0,
        color: utils::colors::Color::WHITE,
    );
    register.insert(crate::key::hud::SHARED_LIVES_LABEL, shared_lives_label);

}

#[derive(Clone, Copy)]
pub struct ShopCardIds {
    pub root: nodus::NodeId,
    pub art: nodus::NodeId,
    pub name: nodus::NodeId,
    pub desc: nodus::NodeId,
    pub price: nodus::NodeId,
    pub sold_overlay: nodus::NodeId,
    pub error_overlay: nodus::NodeId,
    pub sold_text: nodus::NodeId,
}

pub fn init_shop(ui_ctx: &mut nodus::UiContext, register: &mut utils::ids::Register) {
    let shop_root = ui_ctx.add_node(
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
            kind: nodus::VisualKind::None,
            color: utils::colors::Color::new(0, 0, 0, 150),
            visible: false,
            opacity: 1.0,
        },
    );
    register.insert(crate::key::shop::ROOT, shop_root);

    let title_id = nodus::text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(SHOP_TITLE_X, SHOP_TITLE_Y),
        size: nodus::UiVec2::screen(0.3, SHOP_TITLE_FONT_SIZE),
        content: SHOP_TITLE_TEXT,
        font_size: SHOP_TITLE_FONT_SIZE * REFERENCE_H,
        color: utils::colors::Color::GOLD,
    };
    register.insert(crate::key::shop::TITLE, title_id);

    let card_w_unit = nodus::UiUnit::ScreenWidth(SHOP_CARD_W);
    let card_h_unit = nodus::UiUnit::ScreenHeight(SHOP_CARD_H);
    let card_y_unit = nodus::UiUnit::ScreenHeight(SHOP_CARD_Y);
    let gap_unit = (1.0 - card_w_unit * 3.0) / 4.0;

    for i in 0..3 {
        let card_x_unit = gap_unit + (card_w_unit + gap_unit) * (i as f32);

        let card_root = ui_ctx.add_node(
            shop_root,
            nodus::LayoutProps::new(
                nodus::Anchor::TopLeft,
                nodus::UiVec2::new(card_x_unit, card_y_unit),
                nodus::UiVec2::new(card_w_unit, card_h_unit),
            ),
            nodus::VisualProps {
                kind: nodus::VisualKind::Rect,
                color: utils::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let border = nodus::UiUnit::ScreenHeight(SHOP_BORDER_OFFSET);
        let card_inner = ui_ctx.add_node(
            card_root,
            nodus::LayoutProps::new(
                nodus::Anchor::TopLeft,
                nodus::UiVec2::new(nodus::UiUnit::ScreenWidth(SHOP_BORDER_OFFSET), border),
                nodus::UiVec2::new(
                    nodus::UiUnit::ScreenWidth(SHOP_CARD_W - SHOP_BORDER_OFFSET * 2.0),
                    nodus::UiUnit::ScreenHeight(SHOP_CARD_H - SHOP_BORDER_OFFSET * 2.0),
                ),
            ),
            nodus::VisualProps {
                kind: nodus::VisualKind::Rect,
                color: utils::colors::Color::BLACK,
                visible: true,
                opacity: 1.0,
            },
        );

        let art_id = ui_ctx.add_node(
            card_inner,
            nodus::LayoutProps::new(
                nodus::Anchor::TopLeft,
                nodus::UiVec2::screen(SHOP_ART_OFFSET_X, SHOP_ART_OFFSET_Y),
                nodus::UiVec2::screen(SHOP_ART_W, SHOP_ART_H),
            ),
            nodus::VisualProps {
                kind: nodus::VisualKind::Rect,
                color: utils::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let name_id = nodus::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_NAME_OFFSET_Y),
            size: nodus::UiVec2::screen(SHOP_ART_W, SHOP_NAME_FONT_SIZE),
            content: "",
            font_size: SHOP_NAME_FONT_SIZE * REFERENCE_H,
            color: utils::colors::Color::WHITE,
        };

        let desc_id = nodus::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(
                SHOP_TEXT_PADDING_X,
                SHOP_NAME_OFFSET_Y + SHOP_NAME_FONT_SIZE + (4.0 / 1080.0),
            ),
            size: nodus::UiVec2::screen(SHOP_ART_W, 0.05),
            content: "",
            font_size: 0.018 * REFERENCE_H,
            color: utils::colors::Color::LIGHTGRAY,
        };

        let price_id = nodus::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: nodus::Anchor::TopLeft,
            offset: nodus::UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_PRICE_OFFSET_Y),
            size: nodus::UiVec2::screen(SHOP_ART_W, SHOP_PRICE_FONT_SIZE),
            content: "",
            font_size: SHOP_PRICE_FONT_SIZE * REFERENCE_H,
            color: utils::colors::Color::GOLD,
        };

        let sold_overlay_id = ui_ctx.add_node(
            card_inner,
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
                color: utils::colors::Color::new(20, 220, 60, 255),
                visible: false,
                opacity: 0.0,
            },
        );

        let sold_text_id = nodus::text_label! {
            ctx: ui_ctx,
            parent: sold_overlay_id,
            anchor: nodus::Anchor::Center,
            offset: nodus::UiVec2::pixels(30.0, 0.0),
            size: nodus::UiVec2::screen(0.1, 0.03),
            content: "",
            font_size: 35.0,
            color: utils::colors::Color::WHITE,
        };

        let error_overlay_id = ui_ctx.add_node(
            card_inner,
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
                color: utils::colors::Color::new(220, 20, 60, 255),
                visible: false,
                opacity: 0.0,
            },
        );
        let card_id = ShopCardIds {
            root: card_root,
            art: art_id,
            name: name_id,
            desc: desc_id,
            price: price_id,
            sold_overlay: sold_overlay_id,
            error_overlay: error_overlay_id,
            sold_text: sold_text_id,
        };
        register.insert(crate::key::shop::SHOP_CARD_KEYS[i], card_id);
    }

    let close_id = nodus::text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: nodus::Anchor::TopLeft,
        offset: nodus::UiVec2::screen(CLOSE_SHOP_X, CLOSE_SHOP_Y),
        size: nodus::UiVec2::screen(0.2, CLOSE_SHOP_FONT),
        content: "G — Fermer",
        font_size: CLOSE_SHOP_FONT,
        color: utils::colors::Color::GRAY,
    };
    register.insert(crate::key::shop::CLOSE, close_id);
}

pub fn update(
    gui: &mut GuiContext,
    snap: &StateSnapshot,
    bufs: &mut HudBuffers,
    resources: &Resources,
) {
    if let Some(info) = &snap.player_info {
        let ratio = info.health / info.max_health;

        // TODO: Enelver les unwraps aucune raison de panic ici
        bufs.hp.clear();
        write!(bufs.hp, "{} / {}", info.health, info.max_health).unwrap();

        bufs.gold.clear();
        write!(bufs.gold, "Or {}", info.gold).unwrap();

        let wave_info = &snap.wave_info;
        bufs.wave.clear();
        write!(
            bufs.wave,
            "Vague {} | Ennemis {}",
            wave_info.wave_number, wave_info.enemy_remaining
        )
        .unwrap();

        let hp_material_id = match gui
            .ids
            .get::<utils::ids::MaterialId>(crate::key::material::HP_MATERIAL)
        {
            Some(id) => id,
            None => {
                tracing::warn!(
                    "Le material id {} est introuvable dans le register",
                    crate::key::material::HP_MATERIAL
                );
                return;
            }
        };
        if let Some(hp_fill_id) = gui.ids.get::<nodus::NodeId>(crate::key::hud::HP_FILL) {
            // Mise à jour de la barre de vie via uniform_data
            // Le shader hp reçoit un f32 ratio en group(2) binding(0)
            gui.ui_ctx.send_event(nodus::UIEvent::SetMaterial {
                target: hp_fill_id,
                id: hp_material_id,
                texture_id: None,
                uniform_data: bytemuck::cast_slice(&[ratio]).to_vec(),
            });

            gui.ui_ctx.send_event(nodus::UIEvent::SetSize {
                target: hp_fill_id,
                size: nodus::UiVec2::new(
                    nodus::UiUnit::ParentPercent(ratio),
                    nodus::UiUnit::ParentPercent(1.0),
                ),
            });
        } else {
            tracing::warn!("L'id {} est absent du register", crate::key::hud::HP_FILL);
        }

        let hp_text_id = match gui.ids.get::<nodus::NodeId>(crate::key::hud::HP_TEXT) {
            Some(id) => id,
            None => {
                tracing::warn!("L'id {} est absent du register", crate::key::hud::HP_TEXT);
                return;
            }
        };
        gui.ui_ctx.send_event(nodus::UIEvent::SetText {
            target: hp_text_id,
            content: bufs.hp.to_string(),
        });
        let gold_label_id = match gui.ids.get::<nodus::NodeId>(crate::key::hud::GOLD_LABEL) {
            Some(id) => id,
            None => {
                tracing::warn!(
                    "L'id {} est absent du register",
                    crate::key::hud::GOLD_LABEL
                );
                return;
            }
        };
        gui.ui_ctx.send_event(nodus::UIEvent::SetText {
            target: gold_label_id,
            content: bufs.gold.to_string(),
        });
        'wave_label: {
            let wave_label_id = match gui.ids.get::<nodus::NodeId>(crate::key::hud::WAVE_LABEL) {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "L'id {} est absent du register",
                        crate::key::hud::WAVE_LABEL
                    );
                    break 'wave_label;
                }
            };
            gui.ui_ctx.send_event(nodus::UIEvent::SetText {
                target: wave_label_id,
                content: bufs.wave.to_string(),
            });
        }
    }

    'shared_lives: {
        let ui_state = resources.read_resource::<crate::core::ui_state::UiState>();
        let Some(shared_lives) = gui.ids.get(crate::key::hud::SHARED_LIVES_LABEL) else {
            tracing::error!("SHARED_LIVES_LABEL introuvable dans l'id register");
            break 'shared_lives;
        };
        gui.ui_ctx.send_event(nodus::UIEvent::SetText {
            target: shared_lives,
            content: format!("Shared Lives {}", ui_state.shared_lives.current),
        });
    }

    update_respawn_menu(gui, resources);
}

fn update_respawn_menu(gui: &mut GuiContext, resources: &Resources) {
    let game_phase = resources.read_resource::<crate::core::game_phase::GamePhase>();
    if matches!(*game_phase, crate::core::game_phase::GamePhase::Dead) {
        let Some(respawn_label) = gui.ids.get::<nodus::NodeId>(crate::key::hud::RESPAWN_LABEL)
        else {
            tracing::error!("Respawn Label introuvable dans l'id register");
            return;
        };

        let respawn_timer = resources
            .read_resource::<crate::core::ui_state::UiState>()
            .respawn_timer;

        if let Some(timer) = respawn_timer {
            if timer.round() != 0.0 {
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: respawn_label,
                    visible: true,
                });
                gui.ui_ctx.send_event(nodus::UIEvent::SetText {
                    target: respawn_label,
                    content: format!("Respawn disponible dans {} seconde", timer.round() as u32),
                });
            } else {
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: respawn_label,
                    visible: false,
                });
                let shared_lives_button = gui
                    .ids
                    .get::<nodus::NodeId>(crate::key::hud::RESPAWN_SHARED_LIVES_BUTTON)
                    .unwrap();
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: shared_lives_button,
                    visible: true,
                });

                let gold_button = gui
                    .ids
                    .get::<nodus::NodeId>(crate::key::hud::RESPAWN_GOLD_BUTTON)
                    .unwrap();
                gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
                    target: gold_button,
                    visible: true,
                });
            }
        }
    } else {
        let Some(shared_lives_button) = gui
            .ids
            .get::<nodus::NodeId>(crate::key::hud::RESPAWN_SHARED_LIVES_BUTTON)
        else {
            tracing::error!("RESPAWN_SHARED_LIVES_BUTTON introuvable dans l'id register");
            return;
        };
        gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
            target: shared_lives_button,
            visible: false,
        });
        let Some(gold_button) = gui
            .ids
            .get::<nodus::NodeId>(crate::key::hud::RESPAWN_GOLD_BUTTON)
        else {
            tracing::error!("RESPAWN_GOLD_BUTTON introuvable dans l'id register");
            return;
        };
        gui.ui_ctx.send_event(nodus::UIEvent::SetVisible {
            target: gold_button,
            visible: false,
        });
    }
}

pub fn prepare_hud(
    frame: &mut prism::Frame,
    ui_ctx: &mut nodus::UiContext,
    buf: &mut nodus::DrawCommandBuffer,
) {
    ui_ctx.collect(buf);
    buf.sort();
    buf.collect_into(frame);
    buf.clear();
}

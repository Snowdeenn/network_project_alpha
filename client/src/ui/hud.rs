use std::fmt::Write;
use crate::app::states::in_game::{GuiContext, HudBuffers};
use crate::core::config::*;
use utils::ids::ShaderId;
use utils::protocol::StateSnapshot;

pub struct HudIds {
    pub wave_label_id: ui::NodeId,
    pub hp_bg_id: ui::NodeId,
    pub hp_fill_id: ui::NodeId,
    pub hp_text_id: ui::NodeId,
    pub gold_label_id: ui::NodeId,
}

pub fn init_hud(ui_ctx: &mut ui::UiContext, hp_shader_id: ShaderId) -> HudIds {
    let wave_label_id = ui::text_label! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::screen(HUD_PADDING_X, HUD_WAVE_Y),
        size: ui::UiVec2::screen(0.2, 0.05),
        content: "Vague 0 | 0 ennemis",
        font_size: 24.0,
        color: utils
    ::colors::Color::WHITE,
    };

    let (hp_bg_id, hp_fill_id) = ui::progress_bar! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::screen(HUD_PADDING_X, HUD_BAR_Y),
        size: ui::UiVec2::screen(HUD_BAR_W, HUD_BAR_H),
        bg: utils
    ::colors::Color::DARKGRAY,
        fill_color: utils
    ::colors::Color::WHITE,
        shader: hp_shader_id,
    };

    let hp_text_id = ui::text_label! {
        ctx: ui_ctx,
        parent: hp_bg_id,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::pixels(5.0, 2.0),
        size: ui::UiVec2::new(ui::UiUnit::ParentPercent(1.0), ui::UiUnit::ParentPercent(1.0)),
        content: "100/100",
        font_size: 16.0,
        color: utils
    ::colors::Color::WHITE,
    };

    let gold_label_id = ui::text_label! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::screen(HUD_PADDING_X, HUD_GOLD_Y),
        size: ui::UiVec2::screen(0.1, 0.03),
        content: "Or : 0",
        font_size: 24.0,
        color: utils
    ::colors::Color::GOLD,
    };

    HudIds {
        wave_label_id,
        hp_bg_id,
        hp_fill_id,
        hp_text_id,
        gold_label_id,
    }
}

pub struct ShopCardIds {
    pub root: ui::NodeId,
    pub art: ui::NodeId,
    pub name: ui::NodeId,
    pub desc: ui::NodeId,
    pub price: ui::NodeId,
    pub sold_overlay: ui::NodeId,
    pub error_overlay: ui::NodeId,
    pub sold_text: ui::NodeId,
}

pub struct ShopHudIds {
    pub root: ui::NodeId,
    pub cards: [ShopCardIds; 3],
    pub title: ui::NodeId,
    pub close: ui::NodeId,
}

pub fn init_shop(ui_ctx: &mut ui::UiContext) -> ShopHudIds {
    let shop_root = ui_ctx.add_node(
        ui_ctx.root,
        ui::LayoutProps::new(
            ui::Anchor::TopLeft,
            ui::UiVec2::pixels(0.0, 0.0),
            ui::UiVec2::new(ui::UiUnit::ParentPercent(1.0), ui::UiUnit::ParentPercent(1.0)),
        ),
        ui::VisualProps {
            kind: ui::VisualKind::Rect,
            color: utils
        ::colors::Color::new(0, 0, 0, 150),
            visible: false,
            opacity: 1.0,
        },
    );

    let title_id = ui::text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::screen(SHOP_TITLE_X, SHOP_TITLE_Y),
        size: ui::UiVec2::screen(0.3, SHOP_TITLE_FONT_SIZE),
        content: SHOP_TITLE_TEXT,
        font_size: SHOP_TITLE_FONT_SIZE * REFERENCE_H,
        color: utils
    ::colors::Color::GOLD,
    };

    let card_w_unit = ui::UiUnit::ScreenWidth(SHOP_CARD_W);
    let card_h_unit = ui::UiUnit::ScreenHeight(SHOP_CARD_H);
    let card_y_unit = ui::UiUnit::ScreenHeight(SHOP_CARD_Y);

    let gap_unit = (1.0 - card_w_unit * 3.0) / 4.0;

    let mut cards_list = Vec::with_capacity(3);

    for i in 0..3 {
        let card_x_unit = gap_unit + (card_w_unit + gap_unit) * (i as f32);

        let card_root = ui_ctx.add_node(
            shop_root,
            ui::LayoutProps::new(
               ui::Anchor::TopLeft,
                ui::UiVec2::new(card_x_unit, card_y_unit),
                ui::UiVec2::new(card_w_unit, card_h_unit),
            ),
            ui::VisualProps {
                kind: ui::VisualKind::Rect,
                color: utils
            ::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let border = ui::UiUnit::ScreenHeight(SHOP_BORDER_OFFSET);
        let card_inner = ui_ctx.add_node(
            card_root,
            ui::LayoutProps::new(
                ui::Anchor::TopLeft,
                ui::UiVec2::new(ui::UiUnit::ScreenWidth(SHOP_BORDER_OFFSET), border),
                ui::UiVec2::new(
                    ui::UiUnit::ScreenWidth(SHOP_CARD_W - SHOP_BORDER_OFFSET * 2.0),
                    ui::UiUnit::ScreenHeight(SHOP_CARD_H - SHOP_BORDER_OFFSET * 2.0),
                ),
            ),
            ui::VisualProps {
                kind: ui::VisualKind::Rect,
                color: utils
            ::colors::Color::BLACK,
                visible: true,
                opacity: 1.0,
            },
        );

        let art_id = ui_ctx.add_node(
            card_inner,
            ui::LayoutProps::new(
                ui::Anchor::TopLeft,
                ui::UiVec2::screen(SHOP_ART_OFFSET_X, SHOP_ART_OFFSET_Y),
                ui::UiVec2::screen(SHOP_ART_W, SHOP_ART_H),
            ),
            ui::VisualProps {
                kind: ui::VisualKind::Rect,
                color: utils
            ::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let name_id = ui::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: ui::Anchor::TopLeft,
            offset: ui::UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_NAME_OFFSET_Y),
            size: ui::UiVec2::screen(SHOP_ART_W, SHOP_NAME_FONT_SIZE),
            content: "",
            font_size: SHOP_NAME_FONT_SIZE * REFERENCE_H,
            color: utils
        ::colors::Color::WHITE,
        };

        let desc_id = ui::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: ui::Anchor::TopLeft,
            offset: ui::UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_NAME_OFFSET_Y + SHOP_NAME_FONT_SIZE + (4.0 / 1080.0)),
            size: ui::UiVec2::screen(SHOP_ART_W, 0.05),
            content: "",
            font_size: 0.018 * REFERENCE_H,
            color: utils
        ::colors::Color::LIGHTGRAY,
        };

        let price_id = ui::text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: ui::Anchor::TopLeft,
            offset: ui::UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_PRICE_OFFSET_Y),
            size: ui::UiVec2::screen(SHOP_ART_W, SHOP_PRICE_FONT_SIZE),
            content: "",
            font_size: SHOP_PRICE_FONT_SIZE * REFERENCE_H,
            color: utils
        ::colors::Color::GOLD,
        };

        let sold_overlay_id = ui_ctx.add_node(
            card_inner,
            ui::LayoutProps::new(
                ui::Anchor::TopLeft,
                ui::UiVec2::pixels(0.0, 0.0),
                ui::UiVec2::new(ui::UiUnit::ParentPercent(1.0), ui::UiUnit::ParentPercent(1.0)),
            ),
            ui::VisualProps {
                kind: ui::VisualKind::Rect,
                color: utils
            ::colors::Color::new(20, 220, 60, 255),
                visible: false,
                opacity: 0.0,
            },
        );

        let sold_text_id = ui::text_label! {
            ctx: ui_ctx,
            parent: sold_overlay_id,
            anchor: ui::Anchor::Center,
            offset: ui::UiVec2::pixels(30.0, 0.0),
            size: ui::UiVec2::screen(0.1, 0.03),
            content: "",
            font_size: 35.0,
            color: utils
        ::colors::Color::WHITE,
        };

        let error_overlay_id = ui_ctx.add_node(
            card_inner,
            ui::LayoutProps::new(
                ui::Anchor::TopLeft,
                ui::UiVec2::pixels(0.0, 0.0),
                ui::UiVec2::new(ui::UiUnit::ParentPercent(1.0), ui::UiUnit::ParentPercent(1.0)),
            ),
            ui::VisualProps {
                kind: ui::VisualKind::Rect,
                color: utils
            ::colors::Color::new(220, 20, 60, 255),
                visible: false,
                opacity: 0.0,
            },
        );

        cards_list.push(ShopCardIds {
            root: card_root,
            art: art_id,
            name: name_id,
            desc: desc_id,
            price: price_id,
            sold_overlay: sold_overlay_id,
            error_overlay: error_overlay_id,
            sold_text: sold_text_id,
        });
    }

    let close_id = ui::text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: ui::Anchor::TopLeft,
        offset: ui::UiVec2::screen(CLOSE_SHOP_X, CLOSE_SHOP_Y),
        size: ui::UiVec2::screen(0.2, CLOSE_SHOP_FONT),
        content: "G — Fermer",
        font_size: CLOSE_SHOP_FONT,
        color: utils
    ::colors::Color::GRAY,
    };

    let cards = [
        cards_list.remove(0),
        cards_list.remove(0),
        cards_list.remove(0),
    ];

    ShopHudIds {
        root: shop_root,
        cards,
        title: title_id,
        close: close_id,
    }
}

pub fn update(
    gui: &mut GuiContext,
    snap: &StateSnapshot,
    bufs: &mut HudBuffers,
) {
    if let Some(info) = &snap.player_info {
        let ratio = info.health / info.max_health;
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

        gui.ui_ctx.send_event(ui::UIEvent::SetSize {
            target: gui.ids.hud.hp_fill_id,
            size: ui::UiVec2::new(ui::UiUnit::ParentPercent(ratio), ui::UiUnit::ParentPercent(1.0)),
        });
        gui.ui_ctx.send_event(ui::UIEvent::SetText {
            target: gui.ids.hud.hp_text_id,
            content: bufs.hp.to_string(),
        });

        gui.ui_ctx.send_event(ui::UIEvent::SetText {
            target: gui.ids.hud.gold_label_id,
            content: bufs.gold.to_string(),
        });

        if let Some(shader) = gui.shader_manager.get_mut(gui.ids.shader) {
            let loc = shader.get_shader_location("u_ratio");
            shader.set_shader_value(loc, ratio);
        }

        gui.ui_ctx.send_event(ui::UIEvent::SetText {
            target: gui.ids.hud.wave_label_id,
            content: bufs.wave.to_string(),
        });
    }
}

use raylib::prelude::*;
use std::fmt::Write;
use crate::app::states::in_game::{GuiContext, HudBuffers};
use crate::core::config::*;
use shared::ids::ShaderId;
use shared::protocol::StateSnapshot;
use ui::prelude::*;
use ui::*;

pub struct HudIds {
    pub wave_label_id: NodeId,
    pub hp_bg_id: NodeId,
    pub hp_fill_id: NodeId,
    pub hp_text_id: NodeId,
    pub gold_label_id: NodeId,
}

pub fn init_hud(ui_ctx: &mut UiContext, hp_shader_id: ShaderId) -> HudIds {
    let wave_label_id = text_label! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: Anchor::TopLeft,
        offset: UiVec2::screen(HUD_PADDING_X, HUD_WAVE_Y),
        size: UiVec2::screen(0.2, 0.05),
        content: "Vague 0 | 0 ennemis",
        font_size: 24.0,
        color: shared::colors::Color::WHITE,
    };

    let (hp_bg_id, hp_fill_id) = progress_bar! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: Anchor::TopLeft,
        offset: UiVec2::screen(HUD_PADDING_X, HUD_BAR_Y),
        size: UiVec2::screen(HUD_BAR_W, HUD_BAR_H),
        bg: shared::colors::Color::DARKGRAY,
        fill_color: shared::colors::Color::WHITE,
        shader: hp_shader_id,
    };

    let hp_text_id = text_label! {
        ctx: ui_ctx,
        parent: hp_bg_id,
        anchor: Anchor::TopLeft,
        offset: UiVec2::pixels(5.0, 2.0),
        size: UiVec2::new(UiUnit::ParentPercent(1.0), UiUnit::ParentPercent(1.0)),
        content: "100/100",
        font_size: 16.0,
        color: shared::colors::Color::WHITE,
    };

    let gold_label_id = text_label! {
        ctx: ui_ctx,
        parent: ui_ctx.root,
        anchor: Anchor::TopLeft,
        offset: UiVec2::screen(HUD_PADDING_X, HUD_GOLD_Y),
        size: UiVec2::screen(0.1, 0.03),
        content: "Or : 0",
        font_size: 24.0,
        color: shared::colors::Color::GOLD,
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
    pub root: NodeId,
    pub art: NodeId,
    pub name: NodeId,
    pub desc: NodeId,
    pub price: NodeId,
    pub sold_overlay: NodeId,
    pub error_overlay: NodeId,
    pub sold_text: NodeId,
}

pub struct ShopHudIds {
    pub root: NodeId,
    pub cards: [ShopCardIds; 3],
    pub title: NodeId,
    pub close: NodeId,
}

pub fn init_shop(ui_ctx: &mut UiContext) -> ShopHudIds {
    let shop_root = ui_ctx.add_node(
        ui_ctx.root,
        LayoutProps::new(
            Anchor::TopLeft,
            UiVec2::pixels(0.0, 0.0),
            UiVec2::new(UiUnit::ParentPercent(1.0), UiUnit::ParentPercent(1.0)),
        ),
        VisualProps {
            kind: VisualKind::Rect,
            color: shared::colors::Color::new(0, 0, 0, 150),
            visible: false,
            opacity: 1.0,
        },
    );

    let title_id = text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: Anchor::TopLeft,
        offset: UiVec2::screen(SHOP_TITLE_X, SHOP_TITLE_Y),
        size: UiVec2::screen(0.3, SHOP_TITLE_FONT_SIZE),
        content: SHOP_TITLE_TEXT,
        font_size: SHOP_TITLE_FONT_SIZE * REFERENCE_H,
        color: shared::colors::Color::GOLD,
    };

    let card_w_unit = UiUnit::ScreenWidth(SHOP_CARD_W);
    let card_h_unit = UiUnit::ScreenHeight(SHOP_CARD_H);
    let card_y_unit = UiUnit::ScreenHeight(SHOP_CARD_Y);

    let gap_unit = (1.0 - card_w_unit * 3.0) / 4.0;

    let mut cards_list = Vec::with_capacity(3);

    for i in 0..3 {
        let card_x_unit = gap_unit + (card_w_unit + gap_unit) * (i as f32);

        let card_root = ui_ctx.add_node(
            shop_root,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::new(card_x_unit, card_y_unit),
                UiVec2::new(card_w_unit, card_h_unit),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: shared::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let border = UiUnit::ScreenHeight(SHOP_BORDER_OFFSET);
        let card_inner = ui_ctx.add_node(
            card_root,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::new(UiUnit::ScreenWidth(SHOP_BORDER_OFFSET), border),
                UiVec2::new(
                    UiUnit::ScreenWidth(SHOP_CARD_W - SHOP_BORDER_OFFSET * 2.0),
                    UiUnit::ScreenHeight(SHOP_CARD_H - SHOP_BORDER_OFFSET * 2.0),
                ),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: shared::colors::Color::BLACK,
                visible: true,
                opacity: 1.0,
            },
        );

        let art_id = ui_ctx.add_node(
            card_inner,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::screen(SHOP_ART_OFFSET_X, SHOP_ART_OFFSET_Y),
                UiVec2::screen(SHOP_ART_W, SHOP_ART_H),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: shared::colors::Color::DARKGRAY,
                visible: true,
                opacity: 1.0,
            },
        );

        let name_id = text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: Anchor::TopLeft,
            offset: UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_NAME_OFFSET_Y),
            size: UiVec2::screen(SHOP_ART_W, SHOP_NAME_FONT_SIZE),
            content: "",
            font_size: SHOP_NAME_FONT_SIZE * REFERENCE_H,
            color: shared::colors::Color::WHITE,
        };

        let desc_id = text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: Anchor::TopLeft,
            offset: UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_NAME_OFFSET_Y + SHOP_NAME_FONT_SIZE + (4.0 / 1080.0)),
            size: UiVec2::screen(SHOP_ART_W, 0.05),
            content: "",
            font_size: 0.018 * REFERENCE_H,
            color: shared::colors::Color::LIGHTGRAY,
        };

        let price_id = text_label! {
            ctx: ui_ctx,
            parent: card_inner,
            anchor: Anchor::TopLeft,
            offset: UiVec2::screen(SHOP_TEXT_PADDING_X, SHOP_PRICE_OFFSET_Y),
            size: UiVec2::screen(SHOP_ART_W, SHOP_PRICE_FONT_SIZE),
            content: "",
            font_size: SHOP_PRICE_FONT_SIZE * REFERENCE_H,
            color: shared::colors::Color::GOLD,
        };

        let sold_overlay_id = ui_ctx.add_node(
            card_inner,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::pixels(0.0, 0.0),
                UiVec2::new(UiUnit::ParentPercent(1.0), UiUnit::ParentPercent(1.0)),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: shared::colors::Color::new(20, 220, 60, 255),
                visible: false,
                opacity: 0.0,
            },
        );

        let sold_text_id = text_label! {
            ctx: ui_ctx,
            parent: sold_overlay_id,
            anchor: Anchor::Center,
            offset: UiVec2::pixels(30.0, 0.0),
            size: UiVec2::screen(0.1, 0.03),
            content: "",
            font_size: 35.0,
            color: shared::colors::Color::WHITE,
        };

        let error_overlay_id = ui_ctx.add_node(
            card_inner,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::pixels(0.0, 0.0),
                UiVec2::new(UiUnit::ParentPercent(1.0), UiUnit::ParentPercent(1.0)),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: shared::colors::Color::new(220, 20, 60, 255),
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

    let close_id = text_label! {
        ctx: ui_ctx,
        parent: shop_root,
        anchor: Anchor::TopLeft,
        offset: UiVec2::screen(CLOSE_SHOP_X, CLOSE_SHOP_Y),
        size: UiVec2::screen(0.2, CLOSE_SHOP_FONT),
        content: "G — Fermer",
        font_size: CLOSE_SHOP_FONT,
        color: shared::colors::Color::GRAY,
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

        gui.ui_ctx.send_event(UIEvent::SetSize {
            target: gui.ids.hud.hp_fill_id,
            size: UiVec2::new(UiUnit::ParentPercent(ratio), UiUnit::ParentPercent(1.0)),
        });
        gui.ui_ctx.send_event(UIEvent::SetText {
            target: gui.ids.hud.hp_text_id,
            content: bufs.hp.to_string(),
        });

        gui.ui_ctx.send_event(UIEvent::SetText {
            target: gui.ids.hud.gold_label_id,
            content: bufs.gold.to_string(),
        });

        if let Some(shader) = gui.shader_manager.get_mut(gui.ids.shader) {
            let loc = shader.get_shader_location("u_ratio");
            shader.set_shader_value(loc, ratio);
        }

        gui.ui_ctx.send_event(UIEvent::SetText {
            target: gui.ids.hud.wave_label_id,
            content: bufs.wave.to_string(),
        });
    }
}

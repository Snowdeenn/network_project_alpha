use raylib::prelude::*;
use shared::protocol::StateSnapshot;

use crate::config::*;
use crate::event::ClientState;
use crate::renderer::ScreenScale;

pub fn render(
    d: &mut RaylibDrawHandle,
    snap: &StateSnapshot,
    s: &ScreenScale,
    client_state: &ClientState,
) {
    d.draw_text(
        &format!(
            "Vague {} | {} ennemis",
            snap.wave_info.wave_number, snap.wave_info.enemy_remaining
        ),
        s.x(HUD_PADDING_X),
        s.y(HUD_WAVE_Y),
        s.font(HUD_WAVE_FONT),
        Color::WHITE,
    );

    if let Some(info) = &snap.player_info {
        let bar_x = s.x(HUD_PADDING_X);
        let bar_y = s.y(HUD_BAR_Y);
        let bar_max_w = s.w(HUD_BAR_W);
        let bar_h = s.h(HUD_BAR_H);
        let bar_w = (bar_max_w as f32 * (info.health / info.max_health)) as i32;

        d.draw_rectangle(bar_x, bar_y, bar_max_w, bar_h, Color::DARKGRAY);
        d.draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::RED);
        d.draw_text(
            &format!("{}/{}", info.health as i32, info.max_health as i32),
            bar_x + 5,
            bar_y + 2,
            s.font(0.015),
            Color::WHITE,
        );

        d.draw_text(
            &format!("Or : {}", info.gold),
            s.x(HUD_PADDING_X),
            s.y(HUD_GOLD_Y),
            s.font(HUD_GOLD_FONT),
            Color::GOLD,
        );
    }

    // Shared Lives Display
    {
        let lives =  &client_state.ui.shared_lives;
        if lives.max > 0 {
            d.draw_text(
                &format!("Vies : {}/{}", lives.current, lives.max),
                s.x(SHARED_LIVES_X),
                s.y(SHARED_LIVES_Y),
                s.font(SHARED_LIVES_FONT),
                Color::RED,
            );
        }
    }
}

use shared::protocol::{EffectType, ShopItem};

fn with_alpha(c: Color, alpha: u8) -> Color {
    Color::new(c.r, c.g, c.b, alpha)
}

pub fn render_shop(d: &mut RaylibDrawHandle, state: &mut ClientState, s: &ScreenScale) {
    if !state.phase.can_show_shop() {
        return;
    }
    let Some(inventory) = &state.shop_ui.inventory else {
        return;
    };

    d.draw_rectangle(0, 0, s.w as i32, s.h as i32, Color::new(0, 0, 0, 150));
    d.draw_text(
        SHOP_TITLE_TEXT,
        s.x(SHOP_TITLE_X),
        s.y(SHOP_TITLE_Y),
        s.font(SHOP_TITLE_FONT_SIZE),
        Color::GOLD,
    );

    for (slot, item_opt) in inventory.iter().enumerate() {
        let x = s.x(SHOP_SLOTS_X[slot]);

        let sold_ratio = if state.shop_ui.sold_timer[slot] > 0.0 {
            // ratio va de 1.0 → 0.0 car le timer décroit
            Some(state.shop_ui.sold_timer[slot] / SOLD_ANIM_DURATION)
        } else {
            None
        };

        match item_opt {
            Some(item) => {
                render_shop_item(d, x, item, s, sold_ratio);

                if state.shop_ui.error_timer[slot] > 0.0 {
                    render_error_overlay(d, x, state.shop_ui.error_timer[slot], 1.5, s);
                }
            }
            None => {
                render_sold(d, x, s);
            }
        }
    }

    d.draw_text(
        "G — Fermer",
        s.x(CLOSE_SHOP_X),
        s.y(CLOSE_SHOP_Y),
        s.font(CLOSE_SHOP_FONT),
        Color::GRAY,
    );
}

fn render_shop_item(
    d: &mut RaylibDrawHandle,
    x: i32,
    item: &ShopItem,
    s: &ScreenScale,
    sold_ratio: Option<f32>,
) {
    let card_alpha = match sold_ratio {
        None => 255,
        Some(r) if r > 0.66 => 255,
        Some(r) => ((r / 0.66) * 255.0).clamp(0.0, 255.0) as u8,
    };

    render_card(d, x, item, s, card_alpha);

    if let Some(r) = sold_ratio {
        render_sold_overlay(d, x, r, card_alpha, s);
    }
}

fn render_card(d: &mut RaylibDrawHandle, x: i32, item: &ShopItem, s: &ScreenScale, alpha: u8) {
    let card_color = match item.effect_type {
        EffectType::Health => Color::DARKGREEN,
        EffectType::Damage => Color::MAROON,
        EffectType::Speed => Color::DARKBLUE,
        EffectType::Gold => Color::GOLD,
    };

    let card_y = s.y(SHOP_CARD_Y);
    let card_w = s.w(SHOP_CARD_W);
    let card_h = s.h(SHOP_CARD_H);
    let border = s.h(SHOP_BORDER_OFFSET);

    d.draw_rectangle(x, card_y, card_w, card_h, with_alpha(card_color, alpha));
    d.draw_rectangle(
        x + border,
        card_y + border,
        card_w - border * 2,
        card_h - border * 2,
        with_alpha(Color::BLACK, alpha),
    );
    d.draw_rectangle(
        x + s.x(SHOP_ART_OFFSET_X),
        card_y + s.y(SHOP_ART_OFFSET_Y),
        s.w(SHOP_ART_W),
        s.h(SHOP_ART_H),
        with_alpha(Color::DARKGRAY, alpha),
    );

    let pad = s.x(SHOP_TEXT_PADDING_X);
    d.draw_text(
        &item.name,
        x + pad,
        card_y + s.y(SHOP_NAME_OFFSET_Y),
        s.font(SHOP_NAME_FONT_SIZE),
        with_alpha(Color::WHITE, alpha),
    );
    d.draw_text(
        &item.description,
        x + pad,
        card_y + s.y(SHOP_NAME_OFFSET_Y) + s.font(SHOP_NAME_FONT_SIZE) + 4,
        s.font(0.018),
        with_alpha(Color::LIGHTGRAY, alpha),
    );
    d.draw_text(
        &format!("PRIX: {} OR", item.price),
        x + pad,
        card_y + s.y(SHOP_PRICE_OFFSET_Y),
        s.font(SHOP_PRICE_FONT_SIZE),
        with_alpha(Color::GOLD, alpha),
    );
}

fn render_sold(d: &mut RaylibDrawHandle, x: i32, s: &ScreenScale) {
    d.draw_rectangle(
        x,
        s.y(SHOP_CARD_Y),
        s.w(SHOP_CARD_W),
        s.h(SHOP_CARD_H),
        Color::new(40, 40, 40, 255),
    );
    d.draw_text(
        "VENDU",
        x + s.x(SHOP_SOLD_TEXT_OFFSET_X),
        s.y(SHOP_CARD_Y) + s.y(SHOP_SOLD_TEXT_OFFSET_Y),
        s.font(SHOP_SOLD_FONT_SIZE),
        Color::GRAY,
    );
}

fn render_sold_overlay(
    d: &mut RaylibDrawHandle,
    x: i32,
    ratio: f32, // 1.0 → 0.0
    card_alpha: u8,
    s: &ScreenScale,
) {
    let card_y = s.y(SHOP_CARD_Y);
    let card_w = s.w(SHOP_CARD_W);
    let card_h = s.h(SHOP_CARD_H);

    let overlay_alpha = if ratio > 0.66 {
        // Fade in : 1.0→0.66 donne overlay_ratio 0.0→1.0
        let phase_ratio = (1.0 - ratio) / 0.34;
        (phase_ratio * 180.0) as u8
    } else {
        // Suit le fade out de la carte
        ((card_alpha as f32 / 255.0) * 180.0) as u8
    };

    let text_alpha = if ratio > 0.66 {
        let phase_ratio = (1.0 - ratio) / 0.34;
        (phase_ratio * 255.0) as u8
    } else {
        card_alpha
    };

    d.draw_rectangle(
        x,
        card_y,
        card_w,
        card_h,
        Color::new(20, 220, 60, overlay_alpha),
    );

    let text = "VENDU !";
    let font_size = s.font(0.020);
    let text_x = x + (card_w / 2) - (text.len() as i32 * (font_size / 3));
    let text_y = card_y + (card_h / 2) - (font_size / 2);
    d.draw_text(
        text,
        text_x,
        text_y,
        font_size,
        Color::new(255, 255, 255, text_alpha),
    );
}

fn render_error_overlay(
    d: &mut RaylibDrawHandle,
    x: i32,
    current_time: f32,
    max_time: f32,
    s: &ScreenScale,
) {
    let ratio = current_time / max_time;

    let alpha = (ratio * 255.0) as u8;

    let card_y = s.y(SHOP_CARD_Y);
    let card_w = s.w(SHOP_CARD_W);
    let card_h = s.h(SHOP_CARD_H);

    let overlay_alpha = (ratio * 180.0) as u8;
    d.draw_rectangle(
        x,
        card_y,
        card_w,
        card_h,
        Color::new(220, 20, 60, overlay_alpha),
    );

    let text = "OR INSUFFISANT !";
    let font_size = s.font(0.020);

    let text_x = x + (card_w / 2) - (text.len() as i32 * (font_size / 3));
    let text_y = card_y + (card_h / 2) - (font_size / 2);

    d.draw_text(
        text,
        text_x,
        text_y,
        font_size,
        Color::new(255, 255, 255, alpha),
    );
}

use raylib::prelude::*;
use shared::protocol::StateSnapshot;

use crate::config::*;
use crate::event::ClientState;
use crate::renderer::ScreenScale;

pub fn render(d: &mut RaylibDrawHandle, snap: &StateSnapshot, s: &ScreenScale) {
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
}

use shared::protocol::{EffectType, ShopItem};

pub fn render_shop(d: &mut RaylibDrawHandle, state: &ClientState, s: &ScreenScale) {
    if !state.show_shop {
        return;
    }
    let Some(inventory) = &state.curr_inventory else {
        return;
    };

    // fond semi-transparent
    d.draw_rectangle(0, 0, s.w as i32, s.h as i32, Color::new(0, 0, 0, 150));

    // titre
    d.draw_text(
        SHOP_TITLE_TEXT,
        s.x(SHOP_TITLE_X),
        s.y(SHOP_TITLE_Y),
        s.font(SHOP_TITLE_FONT_SIZE),
        Color::GOLD,
    );

    for (slot, item_opt) in inventory.iter().enumerate() {
        let x = s.x(SHOP_SLOTS_X[slot]);
        match item_opt {
            Some(item) => render_card(d, x, item, s),
            None => render_sold(d, x, s),
        }
    }

    d.draw_text(
        "G — Fermer",
        s.x(0.448),
        s.y(0.759),
        s.font(0.022),
        Color::GRAY,
    );
}

fn render_card(d: &mut RaylibDrawHandle, x: i32, item: &ShopItem, s: &ScreenScale) {
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

    d.draw_rectangle(x, card_y, card_w, card_h, card_color);
    d.draw_rectangle(
        x + border,
        card_y + border,
        card_w - border * 2,
        card_h - border * 2,
        Color::BLACK,
    );
    d.draw_rectangle(
        x + s.x(SHOP_ART_OFFSET_X),
        card_y + s.y(SHOP_ART_OFFSET_Y),
        s.w(SHOP_ART_W),
        s.h(SHOP_ART_H),
        Color::DARKGRAY,
    );

    let pad = s.x(SHOP_TEXT_PADDING_X);
    d.draw_text(
        &item.name,
        x + pad,
        card_y + s.y(SHOP_NAME_OFFSET_Y),
        s.font(SHOP_NAME_FONT_SIZE),
        Color::WHITE,
    );
    d.draw_text(
        &item.description,
        x + pad,
        card_y + s.y(SHOP_NAME_OFFSET_Y) + s.font(SHOP_NAME_FONT_SIZE) + 4,
        s.font(0.018),
        Color::LIGHTGRAY,
    );
    d.draw_text(
        &format!("PRIX: {} OR", item.price),
        x + pad,
        card_y + s.y(SHOP_PRICE_OFFSET_Y),
        s.font(SHOP_PRICE_FONT_SIZE),
        Color::GOLD,
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


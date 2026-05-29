use raylib::prelude::*;
use shared::protocol::StateSnapshot;

pub fn render(d: &mut RaylibDrawHandle, snap: &StateSnapshot) {
    // vague + ennemis
    d.draw_text(
        &format!("Vague {} | {} ennemis", snap.wave_info.wave_number, snap.wave_info.enemy_remaining),
        20, 20, 24, Color::WHITE,
    );

    // vie + or
    if let Some(info) = &snap.player_info {
        // barre de vie
        let bar_max_w = 200;
        let bar_h     = 20;
        let bar_x     = 20;
        let bar_y     = 60;
        let bar_w     = (bar_max_w as f32 * (info.health / info.max_health)) as i32;

        d.draw_rectangle(bar_x, bar_y, bar_max_w, bar_h, Color::DARKGRAY);
        d.draw_rectangle(bar_x, bar_y, bar_w,     bar_h, Color::RED);
        d.draw_text(
            &format!("{}/{}", info.health as i32, info.max_health as i32),
            bar_x + 5, bar_y + 2, 16, Color::WHITE,
        );

        // or
        d.draw_text(
            &format!("Or : {}", info.gold),
            20, 90, 24, Color::GOLD,
        );
    }
}
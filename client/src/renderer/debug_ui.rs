// src/debug_ui.rs
use crate::event::{ClientState, DebugMode};
use raylib::prelude::*;

use imgui;

pub fn process_debug(
    ui: &mut imgui::Ui,
    d: &mut RaylibDrawHandle,
    cam: &Camera2D,
    client_state: &mut ClientState,
) {
    let mode = client_state.debug.mode;

    if mode == DebugMode::Off {
        return;
    }

    if mode == DebugMode::Overlay || mode == DebugMode::Interactive {
        let mut d2 = d.begin_mode2D(cam);
        for rect in &client_state.debug.attack_box {
            let angle_deg = rect.dir[1].atan2(rect.dir[0]).to_degrees();
            let raylib_rect = Rectangle {
                x: rect.x,
                y: rect.y,
                width: rect.half_length * 2.0,
                height: rect.half_width * 2.0,
            };
            let origin = Vector2 {
                x: rect.half_length,
                y: rect.half_width,
            };
            d2.draw_rectangle_pro(raylib_rect, origin, angle_deg, Color::new(230, 41, 55, 130));
        }
    }

    if mode == DebugMode::Interactive {
        ui.window("🛠️ Panneau de Contrôle Debug")
            .size([350.0, 180.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mode de Debug actif :");
                ui.text_colored(
                    [0.2, 0.8, 0.2, 1.0],
                    format!("{:?}", client_state.debug.mode),
                );
                ui.separator();

                ui.text(format!(
                    "Attack boxes en mémoire : {}",
                    client_state.debug.attack_box.len()
                ));
                ui.separator();
                ui.spacing();

                if ui.button("Fermer le Debug (F3)") {
                    client_state.debug.cycle();
                }
            });
    }
}

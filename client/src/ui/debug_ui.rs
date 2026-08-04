use crate::core::event::{ClientState, DebugMode};

use imgui;

pub fn process_debug(
    ui: &mut imgui::Ui,
    frame: &mut prism::Frame,
    client_state: &mut ClientState,
) {
    let mode = client_state.debug.mode;

    if mode == DebugMode::Off {
        return;
    }

    if mode == DebugMode::Overlay || mode == DebugMode::Interactive {
        for rect in &client_state.debug.attack_box {
            let angle_deg = rect.dir[1].atan2(rect.dir[0]).to_degrees();
            frame.push_hud(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [rect.x, rect.y],
                    size: [rect.half_length * 2.0, rect.half_width * 2.0],
                    rotation: angle_deg,
                    color: [
                        (230 / 255) as f32,
                        (41 / 255 ) as f32,
                        (55 / 255 ) as f32,
                        (130 / 255) as f32,
                    ],
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 10,
            });
        }

        {
            for collider in &mut client_state.debug.collider.drain(..) {
                frame.push_hud(prism::DrawCommand::Shape {
                    shape: prism::Shape::Quad {
                        pos: [collider.x - 20.0, collider.y - 20.0],
                        size: [40.0, 4.0],
                        rotation: 0.0,
                        color: [0.0, 1.0, 0.0, 0.5],
                        uv: None,
                    },
                    blend: prism::BlendMode::Alpha,
                    layer: 10,
                });
            }
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
                ui.text(format!(
                    "Collider en mémoires: {}",
                    client_state.debug.collider.len()
                ));
                ui.spacing();

                if ui.button("Fermer le Debug (F3)") {
                    client_state.debug.cycle();
                }
            });
    }
}

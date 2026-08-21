use crate::app::input::Input;
use crate::core::client::GameNetClient;
pub enum MenuAction {
    None,
    Solo,
    Multi,
}

pub fn handle_input(
    input_state: &Input,
    client: &mut Option<GameNetClient>,
    client_id: u64,
) -> MenuAction {
    if client.is_some() {
        return MenuAction::None; // connexion déjà en cours
    }

    if input_state.is_just_pressed(winit::keyboard::KeyCode::Enter) {
        *client = Some(GameNetClient::new(client_id));
        return MenuAction::Solo;
    }

    if input_state.is_just_pressed(winit::keyboard::KeyCode::KeyM) {
        *client = Some(GameNetClient::new(client_id));
        return MenuAction::Multi;
    }
    MenuAction::None
}

pub fn render(frame: &mut prism::Frame, s: &crate::rendering::ScreenScale) {
    frame.push_hud(prism::DrawCommand::Text {
        content: "Project Alpha".to_string(),
        pos: [s.x(0.35) as f32, s.y(0.3) as f32],
        size: s.font(0.08) as f32,
        color: [1.0, 1.0, 1.0, 1.0], // BLANC
        layer: 0,
    });
    frame.push_hud(prism::DrawCommand::Text {
        content: "ENTREE — SOLO".to_string(),
        pos: [s.x(0.38) as f32, s.y(0.55) as f32],
        size: s.font(0.03) as f32,
        color: [
            (utils::colors::Color::LIGHTGRAY.r as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.g as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });
    frame.push_hud(prism::DrawCommand::Text {
        content: "M - Multijoueur".to_string(),
        pos: [s.x(0.38) as f32, s.y(0.62) as f32],
        size: s.font(0.03) as f32,
        color: [
            (utils::colors::Color::LIGHTGRAY.r as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.g as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });
    frame.push_hud(prism::DrawCommand::Text {
        content: "ECHAP - Quitter".to_string(),
        pos: [s.x(0.40) as f32, s.y(0.77) as f32],
        size: s.font(0.025) as f32,
        color: [
            (utils::colors::Color::LIGHTGRAY.r as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.g as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });
}

pub fn render_connecting(frame: &mut prism::Frame, s: &crate::rendering::ScreenScale) {
    frame.push_hud(prism::DrawCommand::Text {
        content: "Connecting ...".to_string(),
        pos: [s.x(0.38) as f32, s.y(0.48) as f32],
        size: s.font(0.035) as f32,
        color: [
            (utils::colors::Color::LIGHTGRAY.r as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.g as f32) / 255.0,
            (utils::colors::Color::LIGHTGRAY.b as f32) / 255.0,
            1.0,
        ],
        layer: 0,
    });
}

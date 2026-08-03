use crate::core::client::GameNetClient;
use crate::app::input::Input;
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

    if input_state.is_just_pressed(winit::keyboard::KeyCode::M) {
        *client = Some(GameNetClient::new(client_id));
        return MenuAction::Multi;
    }
    MenuAction::None
}

pub fn render(d: &mut RaylibDrawHandle, s: &crate::rendering::ScreenScale) {
    d.clear_background(Color::BLACK);
    d.draw_text(
        "PROJECT ALPHA",
        s.x(0.35),
        s.y(0.3),
        s.font(0.08),
        Color::WHITE,
    );
    d.draw_text(
        "ENTRÉE — SOLO",
        s.x(0.38),
        s.y(0.55),
        s.font(0.03),
        Color::LIGHTGRAY,
    );

    d.draw_text(
        "M - Multijoueur",
        s.x(0.38),
        s.y(0.62),
        s.font(0.03),
        Color::LIGHTGRAY,
    );
    d.draw_text(
        "ÉCHAP — Quitter",
        s.x(0.40),
        s.y(0.77),
        s.font(0.025),
        Color::GRAY,
    );
}

pub fn render_connecting(d: &mut RaylibDrawHandle, s: &crate::rendering::ScreenScale) {
    d.clear_background(Color::BLACK);
    d.draw_text(
        "Connexion en cours...",
        s.x(0.38),
        s.y(0.48),
        s.font(0.035),
        Color::LIGHTGRAY,
    );
}

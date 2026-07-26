use crate::core::event::ClientState;
use crate::core::client::GameNetClient;
use raylib::prelude::*;
use shared::protocol::InputPacket;
use shared::protocol::{ShopAction, ShopActionKind};

pub fn read_input(rl: &RaylibHandle, tick_id: u64, screen_w: i32, screen_h: i32) -> InputPacket {
    let move_dir = {
        let mut dir = [0.0f32, 0.0f32];
        if rl.is_key_down(KeyboardKey::KEY_D) {
            dir[0] += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            dir[0] -= 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            dir[1] += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_W) {
            dir[1] -= 1.0;
        }
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 0.0 {
            [dir[0] / len, dir[1] / len]
        } else {
            dir
        }
    };

    let mouse = rl.get_mouse_position();
    let aim_dir = {
        let dx = mouse.x - screen_w as f32 / 2.0;
        let dy = mouse.y - screen_h as f32 / 2.0;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            [dx / len, dy / len]
        } else {
            [1.0, 0.0]
        }
    };

    let dash = rl.is_key_pressed(KeyboardKey::KEY_SPACE);
    let attack = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);

    InputPacket {
        tick_id,
        move_dir,
        dash,
        attack,
        spell: None,
        aim_dir,
    }
}

pub enum ShopInputAction {
    Open,
    Close,
    None,
}

pub fn handle_shop_input(
    rl: &RaylibHandle,
    client: &mut GameNetClient,
    state: &mut ClientState,
) -> ShopInputAction {
    if !rl.is_key_pressed(KeyboardKey::KEY_G) {
        return ShopInputAction::None;
    }

    if state.phase.can_show_shop() && !state.shop_ui.is_open() {
        client.send_shop_action(&ShopAction {
            kind: ShopActionKind::Open,
            slot: 0,
        });
        ShopInputAction::Open
    } else if state.shop_ui.is_open() {
        client.send_shop_action(&ShopAction {
            kind: ShopActionKind::Close,
            slot: 0,
        });
        state.close_shop();
        ShopInputAction::Close
    } else {
        ShopInputAction::None
    }
}

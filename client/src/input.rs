use raylib::prelude::*;
use shared::protocol::InputPacket;

pub fn read_input(rl: &RaylibHandle, tick_id: u64, screen_w: i32, screen_h: i32) -> InputPacket {
    let move_dir = {
        let mut dir = [0.0f32, 0.0f32];
        if rl.is_key_down(KeyboardKey::KEY_D) { dir[0] += 1.0; }
        if rl.is_key_down(KeyboardKey::KEY_A) { dir[0] -= 1.0; }
        if rl.is_key_down(KeyboardKey::KEY_S) { dir[1] += 1.0; }
        if rl.is_key_down(KeyboardKey::KEY_W) { dir[1] -= 1.0; }
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 0.0 { [dir[0] / len, dir[1] / len] } else { dir }
    };

    let mouse = rl.get_mouse_position();
    let aim_dir = {
        let dx = mouse.x - screen_w as f32 / 2.0;
        let dy = mouse.y - screen_h as f32 / 2.0;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 { [dx / len, dy / len] } else { [1.0, 0.0] }
    };

    InputPacket {
        tick_id,
        move_dir,
        dash:  rl.is_key_pressed(KeyboardKey::KEY_SPACE),
        spell: None,
        aim_dir,
    }
}
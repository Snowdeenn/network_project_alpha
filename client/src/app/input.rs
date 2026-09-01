use crate::app::resources::Resources;
use crate::core::client::GameNetClient;
use utils::protocol::InputPacket;
use utils::protocol::{ShopAction, ShopActionKind};
use std::collections::HashSet;
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Debug, Clone, Default)]
pub struct Input {
    hash_set_kb: HashSet<KeyCode>,
    hash_set_kb_just_pressed: HashSet<KeyCode>,

    hash_set_mouse: HashSet<MouseButton>,
    mouse_position: (f32, f32),
}

impl Input {
    pub fn new() -> Input {
        Default::default()
    }

    pub fn pressed(&mut self, key_code: KeyCode) {
        self.hash_set_kb.insert(key_code);
        self.hash_set_kb_just_pressed.insert(key_code);
    }

    pub fn released(&mut self, key_code: KeyCode) {
        self.hash_set_kb.remove(&key_code);
    }

    pub fn is_pressed(&self, key_code: KeyCode) -> bool {
        self.hash_set_kb.contains(&key_code)
    }

    pub fn is_just_pressed(&self, key_code: KeyCode) -> bool {
        self.hash_set_kb_just_pressed.contains(&key_code)
    }

    pub fn mouse_pressed(&mut self, button: MouseButton) {
        self.hash_set_mouse.insert(button);
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.hash_set_mouse.remove(&button);
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.hash_set_mouse.contains(&button)
    }

    pub fn is_mousew_released(&self, button: MouseButton) -> bool {
        !self.hash_set_mouse.contains(&button)
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn end_frame(&mut self) {
        self.hash_set_kb_just_pressed.clear();
    }
}

pub fn read_input(input_state: &Input, tick_id: u64, screen_w: i32, screen_h: i32) -> InputPacket {
    let move_dir = {
        let mut dir = [0.0f32, 0.0f32];
        if input_state.is_pressed(winit::keyboard::KeyCode::KeyD) {
            dir[0] += 1.0;
        }
        if input_state.is_pressed(winit::keyboard::KeyCode::KeyA) {
            dir[0] -= 1.0;
        }
        if input_state.is_pressed(winit::keyboard::KeyCode::KeyS) {
            dir[1] += 1.0;
        }
        if input_state.is_pressed(winit::keyboard::KeyCode::KeyW) {
            dir[1] -= 1.0;
        }
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 0.0 {
            [dir[0] / len, dir[1] / len]
        } else {
            dir
        }
    };

    let mouse = input_state.mouse_position();
    let aim_dir = {
        let dx = mouse.0 - screen_w as f32 / 2.0;
        let dy = mouse.1 - screen_h as f32 / 2.0;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            [dx / len, dy / len]
        } else {
            [1.0, 0.0]
        }
    };

    let dash = input_state.is_just_pressed(winit::keyboard::KeyCode::Space);
    let attack = input_state.is_mouse_pressed(winit::event::MouseButton::Left);

    let mut spell: Option<utils::protocol::SpellSlot> = None;
    if input_state.is_just_pressed(winit::keyboard::KeyCode::KeyE) {
        spell = Some(utils::protocol::SpellSlot::First)
    }
    if input_state.is_just_pressed(winit::keyboard::KeyCode::KeyQ) {
        spell = Some(utils::protocol::SpellSlot::Second)
    }
    if input_state.is_just_pressed(winit::keyboard::KeyCode::KeyV) {
        spell = Some(utils::protocol::SpellSlot::Third)
    }
    if input_state.is_just_pressed(winit::keyboard::KeyCode::KeyC) {
        spell = Some(utils::protocol::SpellSlot::Fourth)
    }

    InputPacket {
        tick_id,
        move_dir,
        dash,
        attack,
        spell,
        aim_dir,
    }
}

pub enum ShopInputAction {
    Open,
    Close,
    None,
}

pub fn handle_shop_input(
    input_state: &Input,
    client: &mut GameNetClient,
    resource: &mut Resources,
) -> ShopInputAction {
    if !input_state.is_just_pressed(winit::keyboard::KeyCode::KeyG) {
        return ShopInputAction::None;
    }

    let phase = resource.read_resource::<crate::core::game_phase::GamePhase>();
    let mut shop = resource.write_resource::<crate::core::shop_state::ShopUiState>();
    if phase.can_show_shop() && !shop.is_open() {
        client.send_shop_action(&ShopAction {
            kind: ShopActionKind::Open,
            slot: 0,
        });
        ShopInputAction::Open
    } else if shop.is_open() {
        client.send_shop_action(&ShopAction {
            kind: ShopActionKind::Close,
            slot: 0,
        });
        shop.close();
        ShopInputAction::Close
    } else {
        ShopInputAction::None
    }
}

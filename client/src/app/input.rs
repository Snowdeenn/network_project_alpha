use crate::app::resources::Resources;
use crate::core::client::GameNetClient;
use std::collections::{HashSet, VecDeque};
use std::hash::Hash;
use utils::protocol::InputPacket;
use utils::protocol::{ShopAction, ShopActionKind};
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Debug, Clone)]
struct ButtonState<T> {
    held: HashSet<T>,
    just_pressed: HashSet<T>,
}

impl<T> Default for ButtonState<T> {
    fn default() -> Self {
        Self {
            held: HashSet::new(),
            just_pressed: HashSet::new(),
        }
    }
}

impl<T> ButtonState<T>
where
    T: Copy + Eq + Hash,
{
    fn press(&mut self, button: T) -> bool {
        if !self.held.insert(button) {
            return false;
        }

        self.just_pressed.insert(button);
        true
    }

    fn release(&mut self, button: T) {
        self.held.remove(&button);
    }

    fn is_pressed(&self, button: T) -> bool {
        self.held.contains(&button)
    }

    fn is_just_pressed(&self, button: T) -> bool {
        self.just_pressed.contains(&button)
    }

    fn end_frame(&mut self) {
        self.just_pressed.clear();
    }
}

#[derive(Debug, Clone, Default)]
struct PendingActions {
    dash: bool,
    spells: VecDeque<utils::protocol::SpellSlot>,
}

impl PendingActions {
    fn take_dash(&mut self) -> bool {
        std::mem::take(&mut self.dash)
    }

    fn take_spell(&mut self) -> Option<utils::protocol::SpellSlot> {
        self.spells.pop_front()
    }
}

#[derive(Debug, Clone, Default)]
struct GameplayInput {
    active_slot: Option<utils::protocol::SpellSlot>,
    pending: PendingActions,
}

impl GameplayInput {
    fn toggle_slot(&mut self, slot: utils::protocol::SpellSlot) {
        self.active_slot = match self.active_slot {
            Some(active) if active.index() == slot.index() => None,
            _ => Some(slot),
        };
    }

    fn queue_spell(&mut self) {
        if let Some(slot) = self.active_slot {
            self.pending.spells.push_back(slot);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Input {
    keyboard: ButtonState<KeyCode>,
    mouse: ButtonState<MouseButton>,
    mouse_position: (f32, f32),
    gameplay: GameplayInput,
}

impl Input {
    pub fn new() -> Input {
        Default::default()
    }

    pub fn pressed(&mut self, key_code: KeyCode) {
        if !self.keyboard.press(key_code) {
            return;
        }

        match key_code {
            KeyCode::Space => self.gameplay.pending.dash = true,
            KeyCode::KeyE => self.gameplay.toggle_slot(utils::protocol::SpellSlot::First),
            KeyCode::KeyQ => self
                .gameplay
                .toggle_slot(utils::protocol::SpellSlot::Second),
            KeyCode::KeyV => self.gameplay.toggle_slot(utils::protocol::SpellSlot::Third),
            KeyCode::KeyC => self
                .gameplay
                .toggle_slot(utils::protocol::SpellSlot::Fourth),
            _ => {}
        }
    }

    pub fn released(&mut self, key_code: KeyCode) {
        self.keyboard.release(key_code);
    }

    pub fn is_pressed(&self, key_code: KeyCode) -> bool {
        self.keyboard.is_pressed(key_code)
    }

    pub fn is_just_pressed(&self, key_code: KeyCode) -> bool {
        self.keyboard.is_just_pressed(key_code)
    }

    pub fn mouse_pressed(&mut self, button: MouseButton) {
        if self.mouse.press(button) && button == MouseButton::Left {
            self.gameplay.queue_spell();
        }
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.mouse.release(button);
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse.is_pressed(button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse.is_just_pressed(button)
    }

    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        !self.mouse.is_pressed(button)
    }

    pub fn active_slot(&self) -> Option<utils::protocol::SpellSlot> {
        self.gameplay.active_slot
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn end_frame(&mut self) {
        self.keyboard.end_frame();
        self.mouse.end_frame();
    }
}

pub fn read_input(
    input_state: &mut Input,
    tick_id: u64,
    screen_w: i32,
    screen_h: i32,
) -> InputPacket {
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

    let dash = input_state.gameplay.pending.take_dash();
    let attack = input_state.is_mouse_pressed(winit::event::MouseButton::Left);
    let spell = input_state.gameplay.pending.take_spell();

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

#[cfg(test)]
mod tests {
    use super::*;
    use utils::protocol::SpellSlot;

    fn read_packet(input: &mut Input) -> InputPacket {
        read_input(input, 1, 1920, 1080)
    }

    #[test]
    fn keyboard_just_pressed_only_tracks_the_initial_press() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyW);
        assert!(input.is_pressed(KeyCode::KeyW));
        assert!(input.is_just_pressed(KeyCode::KeyW));

        input.end_frame();
        assert!(input.is_pressed(KeyCode::KeyW));
        assert!(!input.is_just_pressed(KeyCode::KeyW));

        input.pressed(KeyCode::KeyW);
        assert!(!input.is_just_pressed(KeyCode::KeyW));

        input.released(KeyCode::KeyW);
        input.pressed(KeyCode::KeyW);
        assert!(input.is_just_pressed(KeyCode::KeyW));
    }

    #[test]
    fn mouse_just_pressed_only_tracks_the_initial_press() {
        let mut input = Input::new();

        input.mouse_pressed(MouseButton::Left);
        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(input.is_mouse_just_pressed(MouseButton::Left));

        input.end_frame();
        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(!input.is_mouse_just_pressed(MouseButton::Left));

        input.mouse_pressed(MouseButton::Left);
        assert!(!input.is_mouse_just_pressed(MouseButton::Left));

        input.mouse_release(MouseButton::Left);
        input.mouse_pressed(MouseButton::Left);
        assert!(input.is_mouse_just_pressed(MouseButton::Left));
    }

    #[test]
    fn pressing_the_same_spell_key_toggles_its_slot() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyE);
        assert_eq!(input.active_slot().map(SpellSlot::index), Some(0));

        // Une répétition clavier ne doit pas déclencher le toggle.
        input.pressed(KeyCode::KeyE);
        assert_eq!(input.active_slot().map(SpellSlot::index), Some(0));

        input.released(KeyCode::KeyE);
        input.pressed(KeyCode::KeyE);
        assert!(input.active_slot().is_none());
    }

    #[test]
    fn pressing_another_spell_key_switches_the_active_slot() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyE);
        input.pressed(KeyCode::KeyQ);

        assert_eq!(input.active_slot().map(SpellSlot::index), Some(1));
    }

    #[test]
    fn every_spell_key_selects_and_toggles_its_expected_slot() {
        let bindings = [
            (KeyCode::KeyE, SpellSlot::First),
            (KeyCode::KeyQ, SpellSlot::Second),
            (KeyCode::KeyV, SpellSlot::Third),
            (KeyCode::KeyC, SpellSlot::Fourth),
        ];

        for (key, slot) in bindings {
            let mut input = Input::new();

            input.pressed(key);
            assert_eq!(
                input.active_slot().map(SpellSlot::index),
                Some(slot.index())
            );

            input.released(key);
            input.pressed(key);
            assert!(input.active_slot().is_none());
        }
    }

    #[test]
    fn held_movement_and_attack_are_present_in_every_packet() {
        let mut input = Input::new();
        input.pressed(KeyCode::KeyW);
        input.pressed(KeyCode::KeyD);
        input.mouse_pressed(MouseButton::Left);
        input.end_frame();

        for _ in 0..2 {
            let packet = read_packet(&mut input);
            assert!(packet.attack);
            assert!((packet.move_dir[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
            assert!((packet.move_dir[1] + std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
        }
    }

    #[test]
    fn pending_dash_survives_end_frame_and_is_consumed_once() {
        let mut input = Input::new();

        input.pressed(KeyCode::Space);
        input.end_frame();

        assert!(read_packet(&mut input).dash);
        assert!(!read_packet(&mut input).dash);
    }

    #[test]
    fn pending_spell_survives_end_frame_and_is_consumed_once() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyV);
        input.mouse_pressed(MouseButton::Left);
        input.mouse_release(MouseButton::Left);
        input.end_frame();

        assert_eq!(read_packet(&mut input).spell.map(SpellSlot::index), Some(2));
        assert!(read_packet(&mut input).spell.is_none());
        assert_eq!(input.active_slot().map(SpellSlot::index), Some(2));
    }

    #[test]
    fn queued_spell_keeps_the_slot_selected_at_click_time() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyE);
        input.mouse_pressed(MouseButton::Left);
        input.mouse_release(MouseButton::Left);
        input.pressed(KeyCode::KeyQ);

        assert_eq!(input.active_slot().map(SpellSlot::index), Some(1));
        assert_eq!(read_packet(&mut input).spell.map(SpellSlot::index), Some(0));
    }

    #[test]
    fn several_spell_clicks_are_queued_in_order() {
        let mut input = Input::new();

        input.pressed(KeyCode::KeyE);
        input.mouse_pressed(MouseButton::Left);
        input.mouse_release(MouseButton::Left);
        input.pressed(KeyCode::KeyQ);
        input.mouse_pressed(MouseButton::Left);
        input.mouse_release(MouseButton::Left);

        assert_eq!(read_packet(&mut input).spell.map(SpellSlot::index), Some(0));
        assert_eq!(read_packet(&mut input).spell.map(SpellSlot::index), Some(1));
        assert!(read_packet(&mut input).spell.is_none());
    }
}

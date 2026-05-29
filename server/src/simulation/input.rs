// simulation/input.rs
use shared::protocol::InputPacket;

#[derive(Debug, Default)]
pub struct InputQueue(pub Vec<InputPacket>);

#[derive(Debug, Default, Clone, Copy)]
pub struct InputState {
    pub move_dir: [f32; 2],
    pub aim_dir:  [f32; 2],
    pub dash:     bool,
    pub spell:    Option<u8>,
}
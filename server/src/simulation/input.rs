// simulation/input.rs
use shared::protocol::InputPacket;

#[derive(Debug, Default)]
pub struct InputQueue(pub Vec<InputPacket>);

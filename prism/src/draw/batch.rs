use crate::draw::commands::DrawCommand;

#[derive(Debug)]
pub struct DrawCommandBuffer {
    commands: Vec<DrawCommand>,
}

impl DrawCommandBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    pub fn sort(&mut self) {
        self.commands.sort_unstable_by_key(|cmd| sort_key(cmd));
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands[..]
    }
    pub fn commands_mut(&mut self) -> &mut [DrawCommand] {
        &mut self.commands[..]
    }
}

fn sort_key(cmd: &DrawCommand) -> u64 {
    let (layer, blend, shader_id, texture_id) = match cmd {
        DrawCommand::Shape { layer, blend, .. } => (*layer, *blend as u8, 0u16, 0u32),
        DrawCommand::Mesh { layer, blend, .. } => (*layer, *blend as u8, 0u16, 0u32),
        DrawCommand::Texture {
            layer, blend, id, ..
        } => {
            (*layer, *blend as u8, 0u16, id.index as u32)
        }
        DrawCommand::Text { layer, .. } => (*layer, 0u8, 1u16, 0u32),
    };

    ((layer as u64) << 56)
        | ((blend as u64) << 48)
        | ((shader_id as u64) << 32)
        | (texture_id as u64)
}

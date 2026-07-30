use crate::draw::commands::DrawCommand;

pub struct DrawCommandBuffer {
    commands: Vec<DrawCommand>,
}

impl DrawCommandBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { commands: Vec::with_capacity(capacity) }
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
}

fn sort_key(cmd: &DrawCommand) -> u8 {
    match cmd {
        DrawCommand::Mesh {  layer, .. } => *layer,
        DrawCommand::Shape { layer, .. } => *layer,
        DrawCommand::Text {  layer, .. } => *layer,
        DrawCommand::Texture { layer, .. } => *layer,
    }
}
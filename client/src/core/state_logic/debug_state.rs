#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum DebugMode {
    #[default]
    Off,
    Overlay,
    Interactive,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DebugRectState {
    pub x: f32,
    pub y: f32,
    pub half_length: f32,
    pub half_width: f32,
    pub dir: [f32; 2],
    pub lifetime: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DebugCollider {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Default)]
pub struct DebugState {
    pub attack_box: Vec<DebugRectState>,
    pub collider: Vec<DebugCollider>,
    pub hit_pos_anim: [f32; 2],
    pub mode: DebugMode,
    pub cleared: bool,
}

impl DebugState {
    pub fn add_rect(&mut self, x: f32, y: f32, half_length: f32, half_width: f32, dir: [f32; 2]) {
        self.attack_box.push(DebugRectState {
            x,
            y,
            half_length,
            half_width,
            dir,
            lifetime: 0.15,
        });
    }

    pub fn add_collider(&mut self, x: f32, y: f32) {
        self.collider.push(DebugCollider { x, y });
        self.collider.push(DebugCollider { x, y });
    }

    pub fn set_hit_anim(&mut self, pos: [f32; 2]) {
        self.hit_pos_anim = pos;
    }

    pub fn cycle(&mut self) {
        self.mode = match self.mode {
            DebugMode::Off => DebugMode::Overlay,
            DebugMode::Overlay => DebugMode::Interactive,
            DebugMode::Interactive => DebugMode::Off,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.attack_box.retain_mut(|rect| {
            rect.lifetime -= dt;
            rect.lifetime > 0.0
        });
    }
}
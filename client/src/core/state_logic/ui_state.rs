#[derive(Debug)]
pub struct SharedLivesDisplay {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug)]
pub enum SpectatorMode {
    Free,
    Follow { target_id: u64 },
}

#[derive(Debug)]
pub struct UiState {
    pub shared_lives: SharedLivesDisplay,
    pub respawn_timer: Option<f32>,
    pub spectator_mode: Option<SpectatorMode>,
}

impl UiState {
    pub fn update(&mut self, dt: f32) {
        if let Some(ref mut timer) = self.respawn_timer {
            *timer = (*timer - dt).max(0.0);
            if *timer == 0.0 {
                self.respawn_timer = None;
            }
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            shared_lives: SharedLivesDisplay { current: 0, max: 0 },
            respawn_timer: None,
            spectator_mode: None,
        }
    }
}

#[derive(Debug, Default)]
pub enum GamePhase {
    #[default]
    Wave,
    BetweenWave {
        time_remaining: std::time::Duration,
        shop_available: bool,
    },
    Dead,
    Respawning,
    GameOver,
}

impl GamePhase {
    pub fn can_show_shop(&self) -> bool {
        matches!(
            self,
            GamePhase::BetweenWave {
                shop_available: true,
                ..
            }
        )
    }

    pub fn update(&mut self, dt: f32) {
        if let GamePhase::BetweenWave { time_remaining, .. } = self {
            if time_remaining.as_secs_f32() > 0.0 {
                *time_remaining = time_remaining.saturating_sub(std::time::Duration::from_secs_f32(dt));
            }
        }
    }
}
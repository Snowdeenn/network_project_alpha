use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig { 
    pub port: u32, 
    pub tick_rate_hz: u32, 
    pub session_code_length: u32, 
    pub session_code_charset: String, 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhysicsConfig { 
    pub friction: f64, 
    pub spawn_radius: f64, 
    pub knockback_force: f64,
    pub knockback_duration: f64, 
    pub dash_duration_ms: u32, 
    pub dash_cooldown_secs: f64, 
}

#[derive(Debug)]
pub struct SharedLives {
    pub remaining: u32,
    pub max: u32,
}

impl SharedLives {
    pub fn new(max: u32) -> Self {
        Self { remaining: max, max }
    }
}
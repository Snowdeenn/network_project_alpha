use std::time::Duration;
use legion::Entity;

pub struct Player;
pub struct IA;
pub struct Coin;
pub struct Projectile;
pub struct EntityId(pub u64);
pub struct RangedBrain;
pub struct MeleeBrain;
pub struct KamikazeBrain;

#[derive(Debug, Default, Clone, Copy)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Default)]
pub struct Velocity {
    pub dx: f64,
    pub dy: f64,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum DashState {
    Idle,
    Dashing(Duration),
    Cooldown(Duration),
}
#[derive(Debug, PartialEq)]
pub struct Dash(pub DashState);

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, PartialEq)]
pub enum HealthState {
    Alive,
    Dead,
}

#[derive(Debug)]
pub struct Health {
   pub hp: u32,
   pub max_hp: u32,
   pub state: HealthState,
}

#[derive(Debug, PartialEq)]
pub struct Active(pub bool);

pub struct CoinValue(pub u32); 

#[derive(Debug, Default, Clone, Copy)]
pub struct InputState {
    pub move_dir: [f32; 2],
    pub aim_dir:  [f32; 2],
    pub spell:    Option<u8>,
    pub attack:   bool,
    pub dash:     bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub dir: [f32; 2],
    pub half_length: f32,
    pub half_width: f32
}

#[derive(Debug, Clone, Copy)]
pub struct Owner(pub Entity);

#[derive(Debug)]
pub struct AttackTimer {
    pub remaining: Duration,
    pub interval: Duration,
}

pub struct Target(pub Option<Entity>);
pub struct AttackIntent {
    pub aim_dir: [f32; 2],
    pub box_half_length: f64,
    pub box_half_width: f64,
    pub projectile_speed: Option<f64>,
    pub damage: u32,
    pub range: f64,
}

#[derive(Debug)]
pub struct AttackStats {
    pub range: f64,
    pub damage: u32,
    pub box_half_length: f64,
    pub box_half_width: f64,
    pub projectile_speed: Option<f64>,
}

#[derive(Debug)]
pub struct TeamFilter {
    pub is_player: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Damage(pub u32);

#[derive(Debug, PartialEq)]
pub struct Knockback {
    pub dx: f32,
    pub dy: f32,
    pub duration: f32
}

#[derive(Debug, Clone, Copy)]
pub struct MovementStats {
    pub accel: f64,
    pub max_speed: f64,
}

#[derive(Clone, Copy)]
pub struct LifeTime(pub Duration);
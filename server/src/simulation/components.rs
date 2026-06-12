use std::time::Duration;
use legion::Entity;

pub struct Player;
pub struct IA;
pub struct Coin;
pub struct EntityId(pub u64);
pub struct Attacking;

#[derive(Debug, Default)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Geometry {
    pub dir: [f32; 2],
    pub half_length: f32,
    pub half_width: f32
}

#[derive(Debug)]
pub struct Owner(pub Entity);



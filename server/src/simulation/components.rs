use std::time::Duration;

pub struct Player;
pub struct IA;
pub struct Coin;

pub struct EntityId(pub u64);

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


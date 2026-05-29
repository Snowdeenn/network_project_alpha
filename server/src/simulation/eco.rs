use legion::Entity;
use crate::simulation::event::CoinEvent;

#[derive(Debug)]
pub struct CoinPool {
    pub coins: Vec<Entity>,
}

#[derive(Debug)]
pub struct CoinSpawnQueue(pub Vec<CoinEvent>);

#[derive(Debug)]
pub struct PickupQueue(pub Vec<Entity>);

#[derive(Debug)]
pub struct Gold(pub u32);
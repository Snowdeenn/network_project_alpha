use crate::simulation::event::CoinEvent;
use legion::Entity;
use shared::protocol::ShopItem;

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

#[derive(Debug)]
pub struct ItemPool {
    pub items: Vec<Option<ShopItem>>,
}


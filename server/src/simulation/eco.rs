use legion::Entity;
use shared::protocol::ShopItem;
use std::collections::HashMap;
#[derive(Debug)]
pub struct CoinPool {
    pub coins: Vec<Entity>,
}

#[derive(Debug)]
pub struct PlayerGold(pub HashMap<u64, u32>);

impl PlayerGold {
    pub fn new() -> Self { Self(HashMap::new()) }
    pub fn get(&self, client_id: u64) -> u32 { *self.0.get(&client_id).unwrap_or(&0) }
    pub fn add(&mut self, client_id: u64, amount: u32) { *self.0.entry(client_id).or_insert(0) += amount; }
    pub fn sub(&mut self, client_id: u64, amount: u32) { 
        let g = self.0.entry(client_id).or_insert(0);
        *g = g.saturating_sub(amount);
    }
}
#[derive(Debug)]
pub struct ItemPool {
    pub items: Vec<Option<ShopItem>>,
}


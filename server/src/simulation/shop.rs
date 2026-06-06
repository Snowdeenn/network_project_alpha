use rand::prelude::IndexedRandom;
use shared::protocol::ShopItem;
use std::collections::HashMap;

pub struct PlayerShops {
    pub inventories: HashMap<u64, Vec<Option<ShopItem>>>,
}

impl PlayerShops {
    pub fn new() -> Self {
        Self {
            inventories: HashMap::new(),
        }
    }

    pub fn generate(&mut self, player_id: u64, item_pool: &[ShopItem]) -> Vec<Option<ShopItem>> {
        let mut rng = rand::rng();
        let count = item_pool.len().min(3);
        let items: Vec<Option<ShopItem>> = item_pool
            .sample(&mut rng, count)
            .iter()
            .map(|i| Some((*i).clone()))
            .collect();
        self.inventories.insert(player_id, items.clone());
        items
    }

    pub fn buy(&mut self, player_id: u64, slot: usize) -> Option<ShopItem> {
        let inventory = self.inventories.get_mut(&player_id)?;
        inventory.get_mut(slot)?.take()
    }
}

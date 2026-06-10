use rand::{prelude::IndexedRandom};
use shared::protocol::ShopItem;
use std::collections::HashMap;
use crate::simulation::eco::Gold;
use legion::Resources;
pub struct PlayerShops {
    pub inventories: HashMap<u64, Vec<Option<ShopItem>>>,
}

impl PlayerShops {
    pub fn new() -> Self {
        Self {
            inventories: HashMap::new(),
        }
    }

    pub fn generate(&mut self, player_id: u64, item_pool: &[Option<ShopItem>]) -> Vec<Option<ShopItem>> {
        let count = item_pool.len().min(3);
        let items: Vec<Option<ShopItem>> = item_pool
            .sample(&mut rand::rng(), count)
            .cloned()
            .collect();
        self.inventories.insert(player_id, items.clone());
        items
    }

    pub fn buy(&mut self, player_id: u64, slot: usize, res: &mut Resources) -> Option<ShopItem> {
        let inventory = self.inventories.get_mut(&player_id)?;
        let item = inventory.get_mut(slot)?.take();
        if let Some(item) = item {
            if let Some(mut gold) = res.get_mut::<Gold>() {
                if gold.0 >= item.price {
                    gold.0 -= item.price;
                    return Some(item);
                }
            }
        }
        None
    }
}

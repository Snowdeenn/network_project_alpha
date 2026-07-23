use shared::protocol::ShopItem;

#[derive(Debug)]
pub struct ItemPool {
    pub items: Vec<Option<ShopItem>>,
}

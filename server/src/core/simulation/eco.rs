use utils::protocol::ShopItem;

#[derive(Debug)]
pub struct ItemPool {
    pub items: Vec<Option<ShopItem>>,
}

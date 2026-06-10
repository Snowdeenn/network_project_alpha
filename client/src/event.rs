use shared::protocol::{GameEvent, GameEventKind, ShopItem};

pub struct ClientState {
    pub shop_available: bool,
    pub show_shop:      bool,
    pub curr_inventory: Option<Vec<Option<ShopItem>>>,
    pub item_bought:    bool
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            shop_available: false,
            show_shop:      false,
            curr_inventory: None,
            item_bought:    false,
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        match event.kind {
            GameEventKind::ShopOpened { inventory } => {
                self.show_shop      = true;
                self.curr_inventory = Some(inventory);
            },
            GameEventKind::WaveEnd { .. } => {
                self.shop_available = true;
            },
            GameEventKind::WaveStart { .. } => {
                self.shop_available = false;
                self.show_shop      = false;
                self.curr_inventory = None;
            },
            GameEventKind::BossSpawn { .. } => {},
            GameEventKind::PlayerDied { .. } => {},
            GameEventKind::ItemBought { .. } => {
                self.item_bought = true;
            },
            GameEventKind::PurchaseFailed => {},
        }
    }

    pub fn close_shop(&mut self) {
        self.show_shop = false;
    }
}
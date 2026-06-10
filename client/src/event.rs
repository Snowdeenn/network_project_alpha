use shared::protocol::{GameEvent, GameEventKind, ShopItem};
use crate::config::SOLD_ANIM_DURATION;

pub struct ClientState {
    pub shop_available:  bool,
    pub show_shop:       bool,
    pub curr_inventory:  Option<Vec<Option<ShopItem>>>,
    pub error_timers:    Vec<f32>,
    pub sold_timers:     Vec<f32>,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            shop_available:  false,
            show_shop:       false,
            curr_inventory:  None,
            error_timers:    vec![0.0; 3],
            sold_timers:     vec![0.0; 3],
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
                self.sold_timers    = vec![0.0; 3];
            },
            GameEventKind::BossSpawn { .. } => {},
            GameEventKind::PlayerDied { .. } => {},
            GameEventKind::ItemBought { slot } => {
                let slot = slot as usize;
                // On démarre l'animation SANS vider l'item :
                // l'item reste visible pendant le fade out
                if slot < self.sold_timers.len() {
                    self.sold_timers[slot] = SOLD_ANIM_DURATION;
                }
            },
            GameEventKind::PurchaseFailed { slot } => {
                if slot < self.error_timers.len() {
                    self.error_timers[slot] = 1.5;
                }
            },
        }
    }

    pub fn update_timers(&mut self, dt: f32) {
        for timer in self.error_timers.iter_mut() {
            if *timer > 0.0 {
                *timer = (*timer - dt).max(0.0);
            }
        }
        for (slot, timer) in self.sold_timers.iter_mut().enumerate() {
            if *timer > 0.0 {
                *timer = (*timer - dt).max(0.0);

                // Animation terminée → on vide l'item maintenant
                if *timer == 0.0 {
                    if let Some(inv) = &mut self.curr_inventory {
                        if let Some(item_slot) = inv.get_mut(slot) {
                            item_slot.take();
                        }
                    }
                }
            }
        }
    }

    pub fn close_shop(&mut self) {
        self.show_shop = false;
    }
}
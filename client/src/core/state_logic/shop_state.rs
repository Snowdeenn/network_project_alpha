#[derive(Debug, Default)]
pub struct ShopUiState {
    pub inventory: Option<Vec<Option<utils::spell_types::Spell>>>,
    pub error_timer: Vec<f32>,
    pub sold_timer: Vec<f32>,
}

impl ShopUiState {
    pub fn open(&mut self, inventory: Vec<Option<utils::spell_types::Spell>>) {
        self.error_timer = vec![0.0; inventory.len()];
        self.sold_timer = vec![0.0; inventory.len()];
        self.inventory = Some(inventory);
    }

    pub fn close(&mut self) {
        self.inventory = None;
    }

    pub fn item_bought(&mut self, slot: usize) {
        if slot < self.sold_timer.len() {
            self.sold_timer[slot] = crate::core::config::SOLD_ANIM_DURATION;
        }
    }

    pub fn purchase_failed(&mut self, slot: usize) {
        if slot < self.error_timer.len() {
            self.error_timer[slot] = 1.5;
        }
    }

    pub fn is_open(&self) -> bool {
        self.inventory.is_some()
    }

    pub fn update(&mut self, dt: f32) {
        for timer in self.error_timer.iter_mut() {
            if *timer > 0.0 {
                *timer = (*timer - dt).max(0.0);
            }
        }
        for (slot, timer) in self.sold_timer.iter_mut().enumerate() {
            if *timer > 0.0 {
                *timer = (*timer - dt).max(0.0);

                if *timer == 0.0 {
                    if let Some(inv) = &mut self.inventory {
                        if let Some(item_slot) = inv.get_mut(slot) {
                            item_slot.take();
                        }
                    }
                }
            }
        }
    }
}
use std::time::Duration;

use crate::config::SOLD_ANIM_DURATION;
use shared::protocol::{GameEvent, GameEventKind, ShopItem};

pub enum GamePhase {
    Wave,
    BetweenWave {
        time_remaining: Duration,
        shop_available: bool,
    },
    Dead,
}

impl GamePhase {
    pub fn can_show_shop(&self) -> bool {
        matches!(
            self,
            GamePhase::BetweenWave {
                shop_available: true,
                ..
            }
        )
    }

    pub fn update(&mut self, dt: f32) {
        if let GamePhase::BetweenWave { time_remaining, .. } = self {
            if time_remaining.as_secs_f32() > 0.0 {
                *time_remaining = time_remaining.saturating_sub(Duration::from_secs_f32(dt));
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ShopUiState {
    pub inventory: Option<Vec<Option<ShopItem>>>,
    pub error_timer: Vec<f32>,
    pub sold_timer: Vec<f32>,
}

impl ShopUiState {
    pub fn open(&mut self, inventory: Vec<Option<ShopItem>>) {
        self.error_timer = vec![0.0; inventory.len()];
        self.sold_timer = vec![0.0; inventory.len()];
        self.inventory = Some(inventory);
    }

    pub fn close(&mut self) {
        self.inventory = None;
    }

    pub fn item_bought(&mut self, slot: usize) {
        if slot < self.sold_timer.len() {
            self.sold_timer[slot] = SOLD_ANIM_DURATION;
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

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum DebugMode {
    #[default]
    Off,
    Overlay,
    Interactive,
}

#[derive(Debug, Default)]
pub struct DebugRectState {
    pub x: f32,
    pub y: f32,
    pub half_length: f32,
    pub half_width: f32,
    pub dir: [f32; 2],
    pub lifetime: f32,
}

#[derive(Debug, Default)]
pub struct DebugCollider {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Default)]
pub struct DebugState {
    pub attack_box: Vec<DebugRectState>,
    pub collider: Vec<DebugCollider>,
    pub hit_pos_anim: [f32; 2],
    pub mode: DebugMode,
    pub cleared: bool,
}

impl DebugState {
    pub fn add_rect(&mut self, x: f32, y: f32, half_length: f32, half_width: f32, dir: [f32; 2]) {
        self.attack_box.push(DebugRectState {
            x,
            y,
            half_length,
            half_width,
            dir,
            lifetime: 0.15,
        });
    }

    pub fn add_collider(&mut self, x: f32, y: f32) {
        self.collider.push(DebugCollider { x, y});
    }

    pub fn set_hit_anim(&mut self, pos: [f32; 2]) {
        self.hit_pos_anim = pos;
    }

    pub fn cycle(&mut self) {
        self.mode = match self.mode {
            DebugMode::Off => DebugMode::Overlay,
            DebugMode::Overlay => DebugMode::Interactive,
            DebugMode::Interactive => DebugMode::Off,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.attack_box.retain_mut(|rect| {
            rect.lifetime -= dt;
            rect.lifetime > 0.0
        });
    }
}

pub struct ClientState {
    pub phase: GamePhase,
    pub shop_ui: ShopUiState,
    pub debug: DebugState,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Wave,
            shop_ui: ShopUiState::default(),
            debug: DebugState::default(),
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        match event.kind {
            GameEventKind::ShopOpened { inventory } => {
                self.shop_ui.open(inventory);
            }
            GameEventKind::WaveEnd { time_between_wave } => {
                self.phase = GamePhase::BetweenWave {
                    time_remaining: time_between_wave,
                    shop_available: true,
                };
            }
            GameEventKind::WaveStart { .. } => {
                self.phase = GamePhase::Wave;
                self.shop_ui.close();
            }
            GameEventKind::BossSpawn { .. } => {}
            GameEventKind::PlayerDied { .. } => {
                self.phase = GamePhase::Dead;
            }
            GameEventKind::ItemBought { slot } => {
                self.shop_ui.item_bought(slot as usize);
            }
            GameEventKind::PurchaseFailed { slot } => {
                self.shop_ui.purchase_failed(slot as usize);
            }
            GameEventKind::DebugRect {
                x,
                y,
                half_length,
                half_width,
                dir,
            } => {
                self.debug.add_rect(x, y, half_length, half_width, dir);
            }
            GameEventKind::EntityHit { pos } => {
                self.debug.set_hit_anim(pos);
            },
            GameEventKind::DebugCollider { x, y} => {
                if !self.debug.cleared {
                    self.debug.collider.clear();
                    self.debug.cleared = true;
                }
                self.debug.add_collider(x, y);
            }
        }
    }

    pub fn update_timers(&mut self, dt: f32) {
        self.shop_ui.update(dt);
        self.phase.update(dt);
        self.debug.update(dt);
    }

    pub fn close_shop(&mut self) {
        self.shop_ui.close();
    }
}

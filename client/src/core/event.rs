use std::time::Duration;

use crate::core::config::SOLD_ANIM_DURATION;
use utils::protocol::EffectType;
use utils::{
    config::PlayerClass,
    protocol::{GameEvent, GameEventKind, LobbyPhaseInfo, LobbySlotInfo, ShopItem},
};

#[derive(Debug)]
pub enum GamePhase {
    Wave,
    BetweenWave {
        time_remaining: Duration,
        shop_available: bool,
    },
    Dead,
    GameOver,
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

// ===================================================
// ShopUiState
// ====================================================

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

// ===================================================
// Debug State
// ====================================================

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
        self.collider.push(DebugCollider { x, y });
        self.collider.push(DebugCollider { x, y });
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

// ====================================================
// Client State
// ====================================================

#[derive(Debug)]
pub struct ClientState {
    pub phase: GamePhase,
    pub shop_ui: ShopUiState,
    pub debug: DebugState,
    pub ui: UiState,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Wave,
            shop_ui: ShopUiState::default(),
            debug: DebugState::default(),
            ui: UiState::default(),
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
                self.ui.spectator_mode = Some(SpectatorMode::Free);
            }
            GameEventKind::ItemBought { slot } => {
                self.shop_ui.item_bought(slot as usize);
            }
            GameEventKind::PurchaseFailed { slot } => {
                self.shop_ui.purchase_failed(slot as usize);
            }
            GameEventKind::SpawnRect {
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
            }
            GameEventKind::DebugCollider { x, y } => {
                if !self.debug.cleared {
                    self.debug.collider.clear();
                    self.debug.cleared = true;
                }
                self.debug.add_collider(x, y);
            }
            GameEventKind::SharedLivesUpdate { remaining, max } => {
                println!("SharedLivesUpdate reçu");
                self.ui.shared_lives.current = remaining;
                self.ui.shared_lives.max = max;
            }
            GameEventKind::GameOver => {},
            GameEventKind::PlayerHit => (), // On gère ça sur le niveau au dessus
        }
    }

    pub fn update_timers(&mut self, dt: f32) {
        self.shop_ui.update(dt);
        self.phase.update(dt);
        self.debug.update(dt);

        if let Some(ref mut timer) = self.ui.respawn_timer {
            *timer = (*timer - dt).max(0.0);
            if *timer == 0.0 {
                self.ui.respawn_timer = None;
            }
        }
    }

    pub fn close_shop(&mut self) {
        self.shop_ui.close();
    }
}

pub fn handle_shop_ui_event(
    event: &GameEvent,
    ui_ctx: &mut nodus::UiContext,
    shop_ids: &utils::ids::Register,
) {
    let root = match shop_ids.get::<nodus::NodeId>(crate::key::shop::ROOT) {
        Some(id) => id,
        None => {
            tracing::warn!("L'id {} est absent du register", crate::key::shop::ROOT);
            return;
        }
    };
    match &event.kind {
        GameEventKind::ShopOpened { inventory } => {
            // afficher le shop
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: root,
                visible: true,
            });

            // mettre à jour les 3 cartes
            for (slot, item_opt) in inventory.iter().enumerate() {
                let card = match shop_ids
                    .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[slot])
                {
                    Some(id) => id,
                    None => {
                        tracing::warn!(
                            "L'id {} est absent du register",
                            crate::key::shop::SHOP_CARD_KEYS[slot]
                        );
                        return;
                    }
                };
                if let Some(item) = item_opt {
                    let border_color = match item.effect_type {
                        EffectType::Health => utils::colors::Color::DARKGREEN,
                        EffectType::Damage => utils::colors::Color::MAROON,
                        EffectType::Speed => utils::colors::Color::DARKBLUE,
                        EffectType::Gold => utils::colors::Color::GOLD,
                    };
                    ui_ctx.send_event(nodus::UIEvent::SetColor {
                        target: card.root,
                        color: border_color,
                    });
                }
                match item_opt {
                    Some(item) => {
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.name,
                            content: item.name.clone(),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.desc,
                            content: item.description.clone(),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.price,
                            content: format!("PRIX: {} OR", item.price),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetVisible {
                            target: card.sold_overlay,
                            visible: false,
                        });
                    }
                    None => {
                        ui_ctx.send_event(nodus::UIEvent::SetVisible {
                            target: card.sold_overlay,
                            visible: true,
                        });
                    }
                }
            }
        }

        GameEventKind::WaveStart { .. } => {
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: root,
                visible: false,
            });
        }

        GameEventKind::ItemBought { slot } => {
            let card = match shop_ids
                .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[*slot])
            {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "L'id {} est absent du register",
                        crate::key::shop::SHOP_CARD_KEYS[*slot]
                    );
                    return;
                }
            };
            // tween fade sur sold_overlay
            ui_ctx.tween.add(nodus::Tween {
                target: card.sold_overlay,
                property: nodus::TweenProperty::Opacity { from: 0.0, to: 1.0 },
                duration: SOLD_ANIM_DURATION,
                elapsed: 0.0,
                easing: nodus::easing::ease_in_out_quad,
                done: false,
                on_complete: vec![
                    nodus::UIEvent::SetColor {
                        target: card.sold_overlay,
                        color: utils::colors::Color::new(40, 40, 40, 255),
                    },
                    nodus::UIEvent::SetOpacity {
                        target: card.sold_overlay,
                        opacity: 1.0,
                    },
                    nodus::UIEvent::SetText {
                        target: card.sold_text,
                        content: "VENDU".to_string(),
                    },
                    nodus::UIEvent::SetVisible {
                        target: card.sold_text,
                        visible: true,
                    },
                ],
            });
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: card.sold_overlay,
                visible: true,
            });
        }

        GameEventKind::PurchaseFailed { slot } => {
            let card = match shop_ids
                .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[*slot])
            {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "L'id {} est absent du register",
                        crate::key::shop::SHOP_CARD_KEYS[*slot]
                    );
                    return;
                }
            };
            // tween flash rouge
            ui_ctx.tween.add(nodus::Tween {
                target: card.error_overlay,
                property: nodus::TweenProperty::Opacity { from: 0.7, to: 0.0 },
                duration: 1.5,
                elapsed: 0.0,
                easing: nodus::easing::ease_out_quad,
                done: false,
                on_complete: vec![nodus::UIEvent::SetVisible {
                    target: card.error_overlay,
                    visible: false,
                }],
            });
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: card.error_overlay,
                visible: true,
            });
            ui_ctx.send_event(nodus::UIEvent::SetOpacity {
                target: card.error_overlay,
                opacity: 0.7,
            });
        }
        GameEventKind::WaveEnd { .. } => {}
        _ => {}
    }
}

#[derive(Debug)]
pub enum AppScreen {
    MainMenu,
    Lobby(LobbyScreenState),
    InGame(ClientState),
}

#[derive(Debug)]
pub struct LobbyScreenState {
    pub code: String,
    pub slot_index: u8,
    pub slots: Vec<Option<LobbySlotInfo>>,
    pub my_class: Option<PlayerClass>,
    pub ready: bool,
    pub is_solo: bool,
    pub phase: LobbyPhaseInfo,
}

// ===================================================
// Ui State
// ====================================================

#[derive(Debug)]
pub struct SharedLivesDisplay {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug)]
pub enum SpectatorMode {
    Free,
    Follow { target_id: u64 },
}

#[derive(Debug)]
pub struct UiState {
    pub shared_lives: SharedLivesDisplay,
    pub respawn_timer: Option<f32>,
    pub spectator_mode: Option<SpectatorMode>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            shared_lives: SharedLivesDisplay { current: 0, max: 0 },
            respawn_timer: None,
            spectator_mode: None,
        }
    }
}

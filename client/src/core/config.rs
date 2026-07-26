// config.rs

pub const _REFERENCE_W: f32 = 1920.0;
pub const REFERENCE_H: f32 = 1080.0;

// --- SHOP ---
pub const SHOP_CARD_Y: f32 = 290.0 / 1080.0; // 0.268
pub const SHOP_CARD_W: f32 = 350.0 / 1920.0; // 0.182
pub const SHOP_CARD_H: f32 = 500.0 / 1080.0; // 0.463

pub const SHOP_SLOTS_X: [f32; 3] = [
    335.0 / 1920.0,  // 0.174
    785.0 / 1920.0,  // 0.409
    1235.0 / 1920.0, // 0.643
];

pub const SHOP_TITLE_X: f32 = 800.0 / 1920.0; // 0.365
pub const SHOP_TITLE_Y: f32 = 120.0 / 1080.0; // 0.111
pub const SHOP_TITLE_FONT_SIZE: f32 = 64.0 / 1080.0; // 0.059

pub const SHOP_BORDER_OFFSET: f32 = 5.0 / 1080.0;
pub const SHOP_ART_OFFSET_X: f32 = 40.0 / 1920.0;
pub const SHOP_ART_OFFSET_Y: f32 = 40.0 / 1080.0;
pub const SHOP_ART_W: f32 = 270.0 / 1920.0;
pub const SHOP_ART_H: f32 = 200.0 / 1080.0;

pub const SHOP_TEXT_PADDING_X: f32 = 20.0 / 1920.0;
pub const SHOP_NAME_OFFSET_Y: f32 = 260.0 / 1080.0;
pub const SHOP_NAME_FONT_SIZE: f32 = 32.0 / 1080.0;

pub const SHOP_PRICE_OFFSET_Y: f32 = 430.0 / 1080.0;
pub const SHOP_PRICE_FONT_SIZE: f32 = 28.0 / 1080.0;

pub const SHOP_SOLD_TEXT_OFFSET_X: f32 = 120.0 / 1920.0;
pub const SHOP_SOLD_TEXT_OFFSET_Y: f32 = 230.0 / 1080.0;
pub const SHOP_SOLD_FONT_SIZE: f32 = 40.0 / 1080.0;

pub const CLOSE_SHOP_X: f32 = 890.0 / 1920.0;
pub const CLOSE_SHOP_Y: f32 = 820.0 / 1080.0;
pub const CLOSE_SHOP_FONT: f32 = 25.0 / 1080.0;

// --- HUD ---
pub const HUD_PADDING_X: f32 = 20.0 / 1920.0;
pub const HUD_WAVE_Y: f32 = 20.0 / 1080.0;
pub const HUD_WAVE_FONT: f32 = 24.0 / 1080.0;

pub const HUD_BAR_Y: f32 = 60.0 / 1080.0;
pub const HUD_BAR_W: f32 = 200.0 / 1920.0;
pub const HUD_BAR_H: f32 = 20.0 / 1080.0;

pub const HUD_GOLD_Y: f32 = 100.0 / 1080.0;
pub const HUD_GOLD_FONT: f32 = 24.0 / 1080.0;

pub const HUD_SHOP_NOTIF_X: f32 = 775.0 / 1920.0;
pub const HUD_SHOP_NOTIF_Y: f32 = 980.0 / 1080.0;
pub const HUD_SHOP_NOTIF_FONT: f32 = 28.0 / 1080.0;
pub const SHOP_TITLE_TEXT: &str = "BOUTIQUE";

pub const SOLD_ANIM_DURATION: f32 = 1.5;

// --- Timer Between Wave ---
pub const WAVE_TIMER_X: f32 = 690.0 / 1920.0;
pub const WAVE_TIMER_Y: f32 = 60.0 / 1080.0;
pub const WAVE_TIMER_FONT: f32 = 30.0 / 1080.0;

// --- Shared Lives ---
pub const SHARED_LIVES_X: f32 = 1750.0 / 1920.0;
pub const SHARED_LIVES_Y: f32 = 60.0 / 1080.0;
pub const SHARED_LIVES_FONT: f32 = 30.0 / 1080.0;

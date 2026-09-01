pub mod hud {
    pub const ROOT: &str = "hud/root";
    pub const WAVE_LABEL: &str = "hud/wave_label_id";
    pub const HP_BG: &str = "hud/hp_bg_id";
    pub const HP_FILL: &str = "hud/hp_fill_id";
    pub const HP_TEXT: &str = "hud/hp_text_id";
    pub const GOLD_LABEL: &str = "hud/gold_label_id";
    pub const RESPAWN_LABEL: &str = "hud/respawn_label";
    pub const RESPAWN_SHARED_LIVES_BUTTON: &str = "hud/respawn_shared_lives_button";
    pub const RESPAWN_SHARED_LIVES_BUTTON_LABEL: &str = "hud/respawn_shared_lives_button_label";
    pub const RESPAWN_GOLD_BUTTON: &str = "hud/respawn_gold_button";
    pub const RESPAWN_GOLD_BUTTON_LABEL: &str = "hud/respawn_gold_button_label";
    pub const SHARED_LIVES_LABEL: &str = "hud/shared_lives_label";
}
pub mod material {
    pub const HP_MATERIAL: &str = "material/hp_material_id";
}
pub mod shop {
    pub const ROOT: &str = "shop/root";
    pub const TITLE: &str = "shop/title_id";
    pub const SHOP_CARD_KEYS: [&str; 3] = ["shop_card_0", "shop_card_1", "shop_card_2"];
    pub const CLOSE: &str = "shop/close_id";
}
pub mod lobby {
    pub const ROOT: &str = "lobby/root";
    pub const CODE_LABEL: &str = "lobby/code_label";
    pub const SLOT_KEYS: [&str; 4] = ["lobby_slot_0", "lobby_slot_1", "lobby_slot_2", "lobby_slot_3"];
    pub const INSTRUCTION: &str = "lobby/instruction";
    pub const CLASS: &str = "lobby/class";
}
pub mod shader {
    pub const TEXTURED_VERTEX: &str = "shader/textured_vertex";
    pub const TEXTURED_FRAGMENT: &str = "shader/textured_fragment";
    pub const DEFAULT_VERTEX: &str = "shader/default_vertex";
    pub const DEFAULT_FRAGMENT: &str = "shader/default_vertx";
}
pub mod post {
    pub const DEFAULT_POST_VERTEX: &str = "post/default_post_vert";
    pub const DEFAULT_POST_FRAGMENT: &str ="post/default_post_frag";
    pub const HIT_FLASH_FRAG: &str = "post/hit_flash_frag";
}

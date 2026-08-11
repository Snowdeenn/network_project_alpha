pub mod hud {
    pub const ROOT: &'static str = "hud/root";
    pub const WAVE_LABEL: &'static str = "hud/wave_label_id";
    pub const HP_BG: &'static str = "hud/hp_bg_id";
    pub const HP_FILL: &'static str = "hud/hp_fill_id";
    pub const HP_TEXT: &'static str = "hud/hp_text_id";
    pub const GOLD_LABEL: &'static str = "hud/gold_label_id";
}
pub mod material {
    pub const HP_MATERIAL: &'static str = "material/hp_material_id";
}
pub mod shop {
    pub const ROOT: &'static str = "shop/root";
    pub const TITLE: &'static str = "shop/title_id";
    pub const SHOP_CARD_KEYS: [&'static str; 3] = ["shop_card_0", "shop_card_1", "shop_card_2"];
    pub const CLOSE: &'static str = "shop/close_id";
}
pub mod lobby {
    pub const ROOT: &'static str = "lobby/root";
    pub const CODE_LABEL: &'static str = "lobby/code_label";
    pub const SLOT_KEYS: [&'static str; 4] = ["slot_0", "slot_1", "slot_2", "slot_3"];
    pub const INSTRUCTION: &'static str = "lobby/instruction";
    pub const CLASS: &'static str = "lobby/class";
}
pub mod shader {
    pub const TEXTURED_VERTEX: &'static str = "shader/textured_vertex";
    pub const TEXTURED_FRAGEMENT: &'static str = "shader/textured_fragement";
}

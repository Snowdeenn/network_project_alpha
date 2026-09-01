pub mod animation;
pub mod animation_manager;
pub mod asset_manager;
pub mod post_process_effect_type;
pub mod tile_map;

// TODO: move this to a more appropriate place
pub struct ClientSpellSlots {
    pub slots: [Option<utils::protocol::SpellClientConfig>; 4],
}

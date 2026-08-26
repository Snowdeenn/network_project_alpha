pub mod client;
pub mod event;
pub mod config;
mod state_logic;

pub use state_logic::debug_state;
pub use state_logic::game_phase;
pub use state_logic::lobby;
pub use state_logic::shop_state;
pub use state_logic::ui_state;
pub use state_logic::LocalId;
pub use state_logic::screen;


pub fn update_state_timer(app_resources: &mut crate::app::resources::Resources, dt: f32) {
    app_resources.write_resource::<debug_state::DebugState>().update(dt);
    app_resources.write_resource::<shop_state::ShopUiState>().update(dt);
    app_resources.write_resource::<game_phase::GamePhase>().update(dt);
    app_resources.write_resource::<ui_state::UiState>().update(dt);    
}
pub mod debug_state;
pub mod game_phase;
pub mod shop_state;
pub mod ui_state;

pub mod screen {
    use crate::core::state_logic::lobby::LobbyScreenState;
    #[derive(Debug)]
    pub enum AppScreen {
        MainMenu,
        Lobby(LobbyScreenState),
        InGame,
    }
}

pub mod lobby {
    #[derive(Debug)]
    pub struct LobbyScreenState {
        pub code: String,
        pub slot_index: u8,
        pub slots: Vec<Option<utils::protocol::LobbySlotInfo>>,
        pub my_class: Option<utils::config::PlayerClass>,
        pub ready: bool,
        pub is_solo: bool,
        pub phase: utils::protocol::LobbyPhaseInfo,
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalId {
    pub entity_id: u64,
    pub client_id: u64,
}


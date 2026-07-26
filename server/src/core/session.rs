use crate::core::config::ServerConfig;
use rand::seq::IndexedRandom;
use shared::{
    config::{GameConfig, PlayerClass},
    protocol::{LobbyPhaseInfo, LobbySlotInfo},
};

pub enum LobbyPhase {
    Waiting,
    Starting { countdown: std::time::Duration },
    InGame,
}

pub struct LobbySlot {
    pub client_id: u64,
    pub class: Option<PlayerClass>,
    pub ready: bool,
}

pub struct SessionState {
    pub code: String,
    pub slots: [Option<LobbySlot>; 4],
    pub phase: LobbyPhase,
}

#[allow(dead_code)]
impl SessionState {
    pub fn new(server_cfg: &ServerConfig) -> Self {
        Self {
            code: generate_code(server_cfg),
            slots: [const { None }; 4],
            phase: LobbyPhase::Waiting,
        }
    }

    /// Retourne le slot_index attribué, ou None si la session est pleine ou InGame
    pub fn add_slot(&mut self, client_id: u64, game_cfg: &GameConfig) -> Option<u8> {
        let slot_index = self.slots.iter().position(|s| s.is_none())?;
        if slot_index >= game_cfg.max_players as usize {
            return None;
        }

        self.slots[slot_index] = Some(LobbySlot {
            client_id,
            class: None,
            ready: false,
        });

        Some(slot_index as u8)
    }

    pub fn remove_slot(&mut self, client_id: u64) {
        for slot in self.slots.iter_mut() {
            if slot
                .as_ref()
                .map(|s| s.client_id == client_id)
                .unwrap_or(false)
            {
                *slot = None;
                return;
            }
        }
    }

    pub fn set_class(&mut self, client_id: u64, class: PlayerClass) {
        if let Some(slot) = self
            .slots
            .iter_mut()
            .flatten()
            .find(|s| s.client_id == client_id)
        {
            slot.class = Some(class);
        }
    }

    pub fn toggle_ready(&mut self, client_id: u64) {
        if let Some(slot) = self
            .slots
            .iter_mut()
            .flatten()
            .find(|s| s.client_id == client_id)
        {
            // On ne peut pas être ready sans avoir choisi une classe
            if slot.class.is_some() {
                slot.ready = !slot.ready;
            }
        }
    }

    /// Tous les slots ont une classe et sont ready, et il y a au moins 1 joueur
    pub fn all_ready(&self) -> bool {
        let occupied: Vec<_> = self.slots.iter().flatten().collect();
        !occupied.is_empty() && occupied.iter().all(|s| s.ready && s.class.is_some())
    }

    pub fn is_full(&self, game_cfg: &GameConfig) -> bool {
        self.slots.len() >= game_cfg.max_players as usize
    }

    pub fn slot_index_of(&self, client_id: u64) -> Option<u8> {
        self.slots
            .iter()
            .flatten()
            .position(|s| s.client_id == client_id)
            .map(|i| i as u8)
    }

    // Snapshot sérialisable envoyé aux clients via LobbyUpdate
    pub fn to_slot_infos(&self) -> Vec<Option<LobbySlotInfo>> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                slot.as_ref().map(|s| LobbySlotInfo {
                    slot_index: i as u8,
                    player_name: s.client_id.to_string(),
                    class: s.class,
                    ready: s.ready,
                })
            })
            .collect()
    }

    pub fn to_phase_info(&self) -> LobbyPhaseInfo {
        match &self.phase {
            LobbyPhase::Waiting | LobbyPhase::InGame => LobbyPhaseInfo::Waiting,
            LobbyPhase::Starting { countdown } => LobbyPhaseInfo::Starting {
                countdown_secs: countdown.as_secs() as u8,
            },
        }
    }
}

// ---- Génération du code de session ----

fn generate_code(cfg: &ServerConfig) -> String {
    let charset: Vec<char> = cfg.session_code_charset.chars().collect();
    let mut rng = rand::rng();
    (0..cfg.session_code_length)
        .map(|_| *charset.choose(&mut rng).unwrap())
        .collect()
}

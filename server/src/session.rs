use rand::seq::IndexedRandom;
use shared::{
    config::{PlayerClass, GameConfig},
    protocol::{LobbyPhaseInfo, LobbySlotInfo},
};
use crate::config::ServerConfig;

// ---- Structures publiques ----

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
    pub slots: Vec<LobbySlot>,
    pub phase: LobbyPhase,
}

// ---- Implémentation ----

impl SessionState {
    pub fn new(server_cfg: &ServerConfig) -> Self {
        Self {
            code: generate_code(server_cfg),
            slots: Vec::with_capacity(4),
            phase: LobbyPhase::Waiting,
        }
    }

    /// Retourne le slot_index attribué, ou None si la session est pleine ou InGame
    pub fn add_slot(&mut self, client_id: u64, game_cfg: &GameConfig) -> Option<u8> {
        if self.slots.len() >= game_cfg.max_players as usize {
            return None;
        }
        if matches!(self.phase, LobbyPhase::InGame) {
            return None;
        }
        let index = self.slots.len() as u8;
        self.slots.push(LobbySlot {
            client_id,
            class: None,
            ready: false,
        });
        Some(index)
    }

    pub fn remove_slot(&mut self, client_id: u64) {
        self.slots.retain(|s| s.client_id != client_id);
    }

    pub fn set_class(&mut self, client_id: u64, class: PlayerClass) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.client_id == client_id) {
            slot.class = Some(class);
        }
    }

    pub fn toggle_ready(&mut self, client_id: u64) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.client_id == client_id) {
            // On ne peut pas être ready sans avoir choisi une classe
            if slot.class.is_some() {
                slot.ready = !slot.ready;
            }
        }
    }

    /// Tous les slots ont une classe et sont ready, et il y a au moins 1 joueur
    pub fn all_ready(&self) -> bool {
        !self.slots.is_empty()
            && self.slots.iter().all(|s| s.ready && s.class.is_some())
    }

    pub fn is_full(&self, game_cfg: &GameConfig) -> bool {
        self.slots.len() >= game_cfg.max_players as usize
    }

    pub fn slot_index_of(&self, client_id: u64) -> Option<u8> {
        self.slots
            .iter()
            .position(|s| s.client_id == client_id)
            .map(|i| i as u8)
    }

    /// Snapshot sérialisable envoyé aux clients via LobbyUpdate
    pub fn to_slot_infos(&self) -> Vec<Option<LobbySlotInfo>> {
        let mut result: Vec<Option<LobbySlotInfo>> = vec![None; 4];
        for (i, slot) in self.slots.iter().enumerate() {
            result[i] = Some(LobbySlotInfo {
                slot_index: i as u8,
                player_name: slot.client_id.to_string(),
                class: slot.class,
                ready: slot.ready,
            });
        }
        result
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

// ---- Génération du code ----

fn generate_code(cfg: &ServerConfig) -> String {
    let charset: Vec<char> = cfg.session_code_charset.chars().collect();
    let mut rng = rand::rng();
    (0..cfg.session_code_length)
        .map(|_| *charset.choose(&mut rng).unwrap())
        .collect()
}
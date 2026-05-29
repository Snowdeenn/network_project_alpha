use raylib::prelude::*;
use shared::protocol::{EntityKind, StateSnapshot};

pub fn update(cam: &mut Camera2D, snapshot: &StateSnapshot) {
    if let Some(player) = snapshot.entities.iter().find(|e| {
        matches!(e.entity_kind, EntityKind::Player)
    }) {
        cam.target = Vector2::new(player.position[0], player.position[1]);
    }
}
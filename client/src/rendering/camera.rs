use raylib::prelude::*;
use shared::protocol::{EntityKind, StateSnapshot};

pub fn update(cam: &mut Camera2D, prev: Option<&StateSnapshot>, current: &StateSnapshot, t: f32) {
    let curr_player = current
        .entities
        .iter()
        .find(|e| matches!(e.entity_kind, EntityKind::Player));

    let prev_player = prev.and_then(|p| {
        p.entities
            .iter()
            .find(|e| matches!(e.entity_kind, EntityKind::Player))
    });

    if let Some(curr) = curr_player {
        let prev_pos = prev_player.map(|p| p.position).unwrap_or(curr.position);

        cam.target = Vector2::new(
            lerp(prev_pos[0], curr.position[0], t),
            lerp(prev_pos[1], curr.position[1], t),
        );
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

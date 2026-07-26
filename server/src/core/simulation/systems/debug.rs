use crate::{core::queue::Queue, core::simulation::components::*};
use legion::*;
use shared::protocol::{GameEvent, GameEventKind};

#[system(for_each)]
#[filter(component::<Player>())]
pub fn send_collider(pos: &Position, #[resource] game_event_queue: &mut Queue<GameEvent>) {
    game_event_queue.data.push(GameEvent {
        kind: GameEventKind::DebugCollider {
            x: pos.x as f32,
            y: pos.y as f32,
        },
    });
}

#[system(for_each)]
#[filter(component::<Projectile>())] // S'applique uniquement aux projectiles
pub fn debug_projectile_positions(
    pos: &Position,
    geo: &Geometry,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
) {
    // On envoie un point de débug à la position du projectile
    game_event_queue.data.push(GameEvent {
        kind: GameEventKind::SpawnRect {
            x: pos.x as f32,
            y: pos.y as f32,
            half_length: geo.half_length,
            half_width: geo.half_width,
            dir: geo.dir,
        },
    });
}

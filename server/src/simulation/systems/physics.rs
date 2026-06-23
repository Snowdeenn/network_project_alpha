use crate::simulation::components::*;
use crate::simulation::helper::{PlayerPos, Resolution, aabb_overlap, apply_resolution};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use std::time::Duration;

const FRICTION: f64 = 0.85;
const ARENA_W: f64 = 1920.0;
const ARENA_H: f64 = 1080.0;

#[system(for_each)]
pub fn update_position(pos: &mut Position, velo: &Velocity, #[resource] dt: &Duration) {
    pos.x += velo.dx * (*dt).as_secs_f64();
    pos.y += velo.dy * (*dt).as_secs_f64();
}

#[system(for_each)]
#[filter(component::<Player>())]
pub fn update_player_pos(pos: &Position, #[resource] player_pos: &mut PlayerPos) {
    player_pos.x = pos.x;
    player_pos.y = pos.y;
}

#[system(for_each)]
#[filter(component::<Player>())]
#[filter(!component::<Knockback>())]
pub fn update_velocity(
    velo: &mut Velocity,
    state: &InputState,
    mov_stats: &MovementStats,
    #[resource] dt: &Duration,
) {
    let input_x = state.move_dir[0] as f64 * mov_stats.accel * (*dt).as_secs_f64();
    let input_y = state.move_dir[1] as f64 * mov_stats.accel * (*dt).as_secs_f64();

    velo.dx += input_x;
    velo.dy += input_y;
}

#[system(for_each)]
#[filter(!component::<Projectile>())]
pub fn friction(velo: &mut Velocity) {
    velo.dx *= FRICTION;
    velo.dy *= FRICTION;
}

#[system(for_each)]
pub fn collide_arena(pos: &mut Position, col: &Collider) {
    pos.x = pos.x.clamp(0.0, ARENA_W - col.w);
    pos.y = pos.y.clamp(0.0, ARENA_H - col.h);
}

#[system(for_each)]
#[filter(component::<Projectile>())]
pub fn projectile_arena_culling(entity: &Entity, pos: &Position, command: &mut CommandBuffer) {
    const MARGIN: f64 = 100.0;
    if pos.x < -MARGIN || pos.x > ARENA_W + MARGIN || pos.y < -MARGIN || pos.y > ARENA_H + MARGIN {
        command.remove(*entity);
    }
}

#[system]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(IA)]
#[write_component(Velocity)]
#[write_component(Position)]
#[read_component(Active)]
pub fn collide(world: &mut SubWorld) {
    let mut query = <(Entity, &Position, &Collider, &Active)>::query().filter(!component::<Coin>());

    let entities: Vec<_> = query
        .iter(world)
        .filter(|(_, _, _, active)| active.0)
        .map(|(e, p, c, _)| (*e, *p, *c))
        .collect();

    let mut to_resolve: Vec<Resolution> = Vec::new();
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ent_a, pos_a, col_a) = entities[i];
            let (ent_b, pos_b, col_b) = entities[j];

            if let Some((overlap_x, overlap_y)) = aabb_overlap(&pos_a, &col_a, &pos_b, &col_b) {
                let center_a_x = pos_a.x + col_a.w / 2.0;
                let center_b_x = pos_b.x + col_b.w / 2.0;
                let center_a_y = pos_a.y + col_a.h / 2.0;
                let center_b_y = pos_b.y + col_b.h / 2.0;

                to_resolve.push(Resolution {
                    ent_a,
                    ent_b,
                    overlap_x,
                    overlap_y,
                    dir_x: (center_a_x - center_b_x).signum(),
                    dir_y: (center_a_y - center_b_y).signum(),
                    axis: overlap_x < overlap_y,
                });
            }
        }
    }

    for res in to_resolve {
        apply_resolution(world, &res);
    }
}

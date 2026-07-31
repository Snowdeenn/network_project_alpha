use crate::core::simulation::components::*;
use crate::core::simulation::helper::{Resolution, aabb_overlap, apply_resolution};
use crate::core::simulation::spatial_grid::SpatialGrid;
use legion::world::SubWorld;
use legion::*;
use utils::buffer::BufferManager;
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

#[system]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(IA)]
#[write_component(Velocity)]
#[write_component(Position)]
#[read_component(Active)]
pub fn collide(
    world: &mut SubWorld,
    #[resource] buff_manager: &mut BufferManager,
    #[resource] grid: &mut SpatialGrid,
) {
    grid.clear();

    let entities_id = buff_manager.acquire_id::<Vec<(Entity, Position, Collider)>>();
    let candidates_id = buff_manager.acquire_id::<Vec<usize>>();

    let mut query = <(Entity, &Position, &Collider, &Active)>::query().filter(!component::<Coin>());
    {
        let entities = buff_manager
            .get_mut::<Vec<(Entity, Position, Collider)>>(entities_id)
            .expect("[BufferManager] Vec<(Entity, Position, Collider)> introuvable");

        entities.extend(
            query
                .iter(world)
                .filter(|(_, _, _, active)| active.0)
                .map(|(e, p, c, _)| (*e, *p, *c)),
        );

        entities
            .iter()
            .enumerate()
            .for_each(|(i, (_, p, c))| grid.insert(i, p, c));
    }

    grid.build();

    let entities = std::mem::take(
        buff_manager
            .get_mut::<Vec<(Entity, Position, Collider)>>(entities_id)
            .expect("[BufferManager] introuvable"),
    );

    let mut candidates = std::mem::take(
        buff_manager
            .get_mut::<Vec<usize>>(candidates_id)
            .expect("[BufferManager] introuvable"),
    );

    let mut to_resolve: Vec<Resolution> = Vec::new();

    for (i, (entity, pos, col)) in entities.iter().enumerate() {
        grid.query(pos, col, &mut candidates);

        candidates.sort_unstable();
        candidates.dedup();

        for &j in candidates.iter() {
            if j <= i {
                continue;
            }

            let (candidate, candidate_pos, candidate_col) = &entities[j];
            if let Some((overlap_x, overlap_y)) =
                aabb_overlap(pos, col, candidate_pos, candidate_col)
            {
                let center_a_x = pos.x + col.w / 2.0;
                let center_b_x = candidate_pos.x + candidate_col.w / 2.0;
                let center_a_y = pos.y + col.h / 2.0;
                let center_b_y = candidate_pos.y + candidate_col.h / 2.0;

                let diff_x = center_a_x - center_b_x;
                let diff_y = center_a_y - center_b_y;

                to_resolve.push(Resolution {
                    ent_a: *entity,
                    ent_b: *candidate,
                    overlap_x,
                    overlap_y,
                    dir_x: if diff_x == 0.0 { 1.0 } else { diff_x.signum() },
                    dir_y: if diff_y == 0.0 { 1.0 } else { diff_y.signum() },
                    axis: overlap_x < overlap_y,
                });
            }
        }
        candidates.clear();
    }

    if !to_resolve.is_empty() {
        println!("Résolutions détectées : {}", to_resolve.len());
    }
    for res in &to_resolve {
        apply_resolution(world, res);
    }

    to_resolve.clear();
    *buff_manager
        .get_mut::<Vec<(Entity, Position, Collider)>>(entities_id)
        .unwrap() = entities;

    *buff_manager.get_mut::<Vec<usize>>(candidates_id).unwrap() = candidates;

    // Release qui va clear() les vecteurs tout en conservant leur capacity
    buff_manager.release(entities_id);
    buff_manager.release(candidates_id);
}

use legion::EntityStore;
use legion::{Entity, world::SubWorld};

use crate::simulation::components::{Collider, Position, Velocity};
use crate::EnemyDiedQueue;
use crate::InputQueue;
use crate::DamageQueue;
use crate::CoinSpawnQueue;
use crate::PickupQueue;
use crate::GameEventQueue;

#[derive(Debug)]
pub struct Resolution {
    pub ent_a: Entity,
    pub ent_b: Entity,
    pub overlap_x: f64,
    pub overlap_y: f64,
    pub dir_x: f64,
    pub dir_y: f64,
    pub axis: bool,
}

#[derive(Debug)]
pub struct PlayerPos {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct PlayerHp {
    pub hp: f64,
    pub max_hp: f64,
}


pub fn aabb_overlap(
    pos_a: &Position,
    col_a: &Collider,
    pos_b: &Position,
    col_b: &Collider,
) -> Option<(f64, f64)> {
    let r1 = pos_a.x + col_a.w;
    let r2 = pos_b.x + col_b.w;
    let b1 = pos_a.y + col_a.h;
    let b2 = pos_b.y + col_b.h;

    if pos_a.x < r2 && r1 > pos_b.x && pos_a.y < b2 && b1 > pos_b.y {
        let overlap_x = r1.min(r2) - pos_a.x.max(pos_b.x);
        let overlap_y = b1.min(b2) - pos_a.y.max(pos_b.y);

        Some((overlap_x, overlap_y))
    } else {
        None
    }
}

const MIN_BOUNCE: f64 = 50.0;

pub fn apply_resolution(world: &mut SubWorld, res: &Resolution) {
    let epsilon = 0.2;

    if let Ok(mut entry_a) = world.entry_mut(res.ent_a) {
        if res.axis {
            if let Ok(pos) = entry_a.get_component_mut::<Position>() {
                pos.x += (res.overlap_x / 2.0 + epsilon) * res.dir_x;
            }
            if let Ok(velo) = entry_a.get_component_mut::<Velocity>() {
                velo.dx = velo.dx.abs().max(MIN_BOUNCE) * res.dir_x;
            }
        } else {
            if let Ok(pos) = entry_a.get_component_mut::<Position>() {
                pos.y += (res.overlap_y / 2.0 + epsilon) * res.dir_y;
            }
            if let Ok(velo) = entry_a.get_component_mut::<Velocity>() {
                velo.dy = velo.dy.abs().max(MIN_BOUNCE) * res.dir_y;
            }
        }
    }

    if let Ok(mut entry_b) = world.entry_mut(res.ent_b) {
        if res.axis {
            if let Ok(pos) = entry_b.get_component_mut::<Position>() {
                pos.x -= (res.overlap_x / 2.0 + epsilon) * res.dir_x;
            }
            if let Ok(velo) = entry_b.get_component_mut::<Velocity>() {
                velo.dx = velo.dx.abs() * -res.dir_x;
            }
        } else {
            if let Ok(pos) = entry_b.get_component_mut::<Position>() {
                pos.y -= (res.overlap_y / 2.0 + epsilon) * res.dir_y;
            }
            if let Ok(velo) = entry_b.get_component_mut::<Velocity>() {
                velo.dy = velo.dy.abs() * -res.dir_y;
            }
        }
    }
}

pub fn clear_resource_queues(resources: &mut legion::Resources) {
    if let Some(mut queue) = resources.get_mut::<InputQueue>() {
        queue.0.clear();
    }
    if let Some(mut queue) = resources.get_mut::<DamageQueue>() {
        queue.0.clear();
    }
    if let Some(mut queue) = resources.get_mut::<EnemyDiedQueue>() {
        queue.0.clear();
    }
    if let Some(mut queue) = resources.get_mut::<CoinSpawnQueue>() {
        queue.0.clear();
    }
    if let Some(mut queue) = resources.get_mut::<PickupQueue>() {
        queue.0.clear();
    }
}

use crate::navigation::SpatialGrid;
use crate::simulation::resources::components::{Collider, Geometry, PendingAoe, Position};
use legion::{Entity, world::SubWorld};
use utils::spell_types::AoeSpellShape;

enum TargetShape {
    Point,
    Circle,
    Box(Geometry),
    Cone {
        direction: [f64; 2],
        cos_half_angle: f64,
    },
}

/// Géométrie préparée une fois par sort, avant de parcourir ses cibles.
struct TargetArea {
    center: Position,
    extent: f64,
    shape: TargetShape,
}

impl TargetArea {
    fn from_pending(pending: &PendingAoe) -> Self {
        let center_at = |offset: utils::math::Vec2| Position {
            x: (pending.origin[0] + offset.x) as f64,
            y: (pending.origin[1] + offset.y) as f64,
        };
        match pending.aoe {
            None => Self {
                center: Position {
                    x: pending.origin[0] as f64,
                    y: pending.origin[1] as f64,
                },
                extent: 0.0,
                shape: TargetShape::Point,
            },
            Some(AoeSpellShape::Circle { offset, radius }) => Self {
                center: center_at(offset),
                extent: radius as f64,
                shape: TargetShape::Circle,
            },
            Some(AoeSpellShape::Box {
                offset,
                size,
                rotation,
            }) => {
                let geometry = Geometry {
                    half_length: size.x / 2.0,
                    half_width: size.y / 2.0,
                    dir: [rotation.cos(), rotation.sin()],
                };
                Self {
                    center: center_at(offset),
                    extent: (geometry.half_width + geometry.half_length) as f64,
                    shape: TargetShape::Box(geometry),
                }
            }
            Some(AoeSpellShape::Cone {
                offset,
                direction,
                angle,
                range,
            }) => Self {
                center: center_at(offset),
                extent: range as f64,
                shape: TargetShape::Cone {
                    direction: [direction.x as f64, direction.y as f64],
                    cos_half_angle: ((angle / 2.0).to_radians() as f64).cos(),
                },
            },
        }
    }

    fn search_bounds(&self) -> (Position, Collider) {
        (
            Position {
                x: self.center.x - self.extent,
                y: self.center.y - self.extent,
            },
            Collider {
                w: self.extent * 2.0,
                h: self.extent * 2.0,
            },
        )
    }

    fn contains(&self, pos: &Position, collider: &Collider) -> bool {
        let dx = pos.x - self.center.x;
        let dy = pos.y - self.center.y;
        let distance_squared = dx * dx + dy * dy;
        match &self.shape {
            TargetShape::Point => {
                self.center.x >= pos.x
                    && self.center.x <= pos.x + collider.w
                    && self.center.y >= pos.y
                    && self.center.y <= pos.y + collider.h
            }
            TargetShape::Circle => distance_squared <= self.extent * self.extent,
            TargetShape::Box(geometry) => {
                crate::utils::obb_vs_aabb(&self.center, geometry, pos, collider)
            }
            TargetShape::Cone {
                direction,
                cos_half_angle,
            } => {
                if distance_squared > self.extent * self.extent {
                    return false;
                }
                let distance = distance_squared.sqrt();
                if distance < 0.001 {
                    return true;
                }
                let dot = (dx / distance) * direction[0] + (dy / distance) * direction[1];
                dot >= *cos_half_angle
            }
        }
    }
}

pub(super) fn select_aoe_targets(
    world: &SubWorld,
    pending: &PendingAoe,
    grid: &SpatialGrid,
    victims: &[(Entity, Collider, Position)],
    candidates: &mut Vec<usize>,
) -> Vec<Entity> {
    let area = TargetArea::from_pending(pending);
    let (pos, collider) = area.search_bounds();
    candidates.clear();
    grid.query(&pos, &collider, candidates);
    candidates.sort_unstable();
    candidates.dedup();

    let hits = candidates.iter().filter_map(|&idx| {
        let (entity, collider, pos) = &victims[idx];
        if *entity == pending.owner || super::same_team(world, *entity, pending.caster_is_player) {
            return None;
        }
        area.contains(pos, collider).then_some(*entity)
    });
    // Sans forme de zone, seule la première cible contenant le point visé est touchée.
    let limit = if pending.aoe.is_none() { 1 } else { usize::MAX };
    hits.take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::resources::components::Player;
    use legion::World;
    use utils::math::Vec2;

    #[test]
    fn targeting_preserves_team_filter_order_and_unique_hits_for_each_shape() {
        let mut world = World::default();
        let owner = world.push((Player,));
        let ally = world.push((Player,));
        let first = world.push(());
        let second = world.push(());
        let side = world.push(());
        let far = world.push(());
        let collider = Collider { w: 40.0, h: 40.0 };
        let victims = [
            (owner, collider, Position { x: 100.0, y: 100.0 }),
            (ally, collider, Position { x: 100.0, y: 100.0 }),
            (first, collider, Position { x: 100.0, y: 100.0 }),
            (second, collider, Position { x: 120.0, y: 120.0 }),
            (
                side,
                Collider { w: 10.0, h: 10.0 },
                Position { x: 80.0, y: 120.0 },
            ),
            (far, collider, Position { x: 300.0, y: 300.0 }),
        ];
        // Chaque grande cible occupe plusieurs cellules : elle ne doit être touchée qu'une fois.
        let mut grid = SpatialGrid::new(16.0, 500.0, 500.0);
        for (idx, (_, collider, pos)) in victims.iter().enumerate() {
            grid.insert(idx, pos, collider);
        }
        grid.build();
        let (subworld, _) = world.split::<&Player>();
        let cases = [
            ([130.0, 130.0], None, vec![first]),
            (
                [100.0, 100.0],
                Some(AoeSpellShape::Circle {
                    offset: Vec2::new(20.0, 20.0),
                    radius: 30.0,
                }),
                vec![first, second],
            ),
            (
                [100.0, 100.0],
                Some(AoeSpellShape::Box {
                    offset: Vec2::zero(),
                    size: Vec2::new(40.0, 40.0),
                    rotation: std::f32::consts::FRAC_PI_2,
                }),
                vec![first, second],
            ),
            (
                [100.0, 100.0],
                Some(AoeSpellShape::Cone {
                    offset: Vec2::zero(),
                    direction: Vec2::new(1.0, 0.0),
                    angle: 60.0,
                    range: 40.0,
                }),
                vec![first],
            ),
        ];
        let mut candidates = vec![usize::MAX];
        for (origin, aoe, expected) in cases {
            let pending = PendingAoe {
                origin,
                aoe,
                owner,
                aim_dir: [1.0, 0.0],
                effects: vec![],
                caster_is_player: true,
            };
            assert_eq!(
                select_aoe_targets(&subworld, &pending, &grid, &victims, &mut candidates),
                expected
            );
        }
    }
}

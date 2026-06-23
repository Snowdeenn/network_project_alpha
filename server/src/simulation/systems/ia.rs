use legion::*;
use legion::world::SubWorld;
use crate::simulation::components::*;

#[system]
#[read_component(Player)]
#[read_component(Position)]
#[write_component(Target)]
#[read_component(Active)]
#[read_component(IA)]
pub fn ia_targeting(world: &mut SubWorld) {
    let mut player_query = <(Entity, &Position)>::query().filter(component::<Player>());
    let players: Vec<(Entity, Position)> =
        player_query.iter(world).map(|(e, p)| (*e, *p)).collect();

    let mut ia_query = <(&Position, &mut Target, &Active)>::query().filter(component::<IA>());

    for (ia_pos, target, active) in ia_query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        let mut closest_player = None;
        let mut min_dist = f64::MAX;

        for (p_entt, p_pos) in &players {
            let dx = p_pos.x - ia_pos.x;
            let dy = p_pos.y - ia_pos.y;
            let dist = dx * dx + dy * dy;

            if dist < min_dist {
                min_dist = dist;
                closest_player = Some(*p_entt);
            }
        }

        target.0 = closest_player;
    }
}

#[system]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Target)]
#[write_component(Velocity)]
pub fn ia_classic_movement(world: &mut SubWorld) {
    let player_positions: std::collections::HashMap<Entity, Position> = {
        let mut player_query = <(Entity, &Position)>::query()
            .filter(component::<Player>())
            .filter(!component::<Knockback>());
        player_query
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos))
            .collect()
    };

    let mut query =
        <(&Position, &Active, &Target, &mut Velocity)>::query().filter(component::<IA>());

    for (ia_pos, active, target, velo) in query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        if let Some(target_entity) = target.0 {
            if let Some(target_pos) = player_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > 45.0 {
                    velo.dx = (dx / distance) * 140.0;
                    velo.dy = (dy / distance) * 140.0;
                } else {
                    velo.dx = 0.0;
                    velo.dy = 0.0;
                }
            }
        } else {
            velo.dx = 0.0;
            velo.dy = 0.0;
        }
    }
}
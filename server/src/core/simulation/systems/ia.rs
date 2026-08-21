use crate::core::flow_field_manager::FlowFieldManager;
use crate::core::queue::Queue;
use crate::core::simulation::components::*;
use crate::core::simulation::event::DamageEvent;
use arrayvec::ArrayVec;
use legion::world::SubWorld;
use legion::*;
use utils::map::grid::Grid;
use std::collections::HashMap;
use utils::buffer::BufferManager;

#[system]
#[read_component(Player)]
#[read_component(Position)]
#[write_component(Target)]
#[read_component(Active)]
#[read_component(IA)]
pub fn ia_targeting(world: &mut SubWorld, #[resource] buff_manager: &mut BufferManager) {
    let (players_id, players) = buff_manager.acquire::<Vec<(Entity, Position)>>().expect(
        "[BufferManager] devrait retourner un tuple avec l'id et le Vec<(Entity, Position)>",
    );
    players.extend(
        <(Entity, &Position)>::query()
            .filter(component::<Player>())
            .iter(world)
            .map(|(e, p)| (*e, *p)),
    );

    let mut ia_query = <(&Position, &mut Target, &Active)>::query().filter(component::<IA>());

    for (ia_pos, target, active) in ia_query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        let mut closest_player = None;
        let mut min_dist = f64::MAX;

        for (p_entt, p_pos) in players.iter() {
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
    buff_manager.release(players_id);
}

#[system]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Target)]
#[write_component(Velocity)]
#[read_component(MeleeBrain)]
#[read_component(AttackStats)]
#[read_component(MovementStats)]
#[filter(component::<IA>() & (component::<MeleeBrain>() | component::<KamikazeBrain>()))]
pub fn melee_ia_movement(
    world: &mut SubWorld,
    #[resource] buff_manager: &mut BufferManager,
    #[resource] flow_field_manager: &FlowFieldManager,
    #[resource] grid: &Grid,
    query: &mut Query<(
        &Position,
        &Active,
        &Target,
        &mut Velocity,
        &AttackStats,
        &MovementStats,
    )>,
) {
    let (player_pos_id, player_positions) =
        buff_manager.acquire::<HashMap<Entity, Position>>().expect(
            "[BufferManager] devrait retourner un tuple avec l'id et le HashMap<Entity, Position>",
        );
    player_positions.extend(
        <(Entity, &Position)>::query()
            .filter(component::<Player>())
            .filter(!component::<Knockback>())
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos)),
    );

    for (ia_pos, active, target, velo, stats, mov_stats) in query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        if let Some(target_entity) = target.0 {
            if let Some(target_pos) = player_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > (stats.range - 5.0) {
                   let ia_vec = utils::math::Vec2::new(ia_pos.x as f32, ia_pos.y as f32);
                    
                    // On récupère le vecteur de direction qui évite les murs
                    let dir = flow_field_manager.get_direction(grid, target_entity, ia_vec);
                    let speed = mov_stats.accel.clamp(-mov_stats.max_speed, mov_stats.max_speed);

                    velo.dx = dir.x as f64 * speed;
                    velo.dy = dir.y as f64 * speed;
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
    buff_manager.release(player_pos_id);
}

#[system]
#[read_component(IA)]
#[read_component(RangedBrain)]
#[read_component(Position)]
#[read_component(Active)]
#[read_component(Target)]
#[read_component(Knockback)]
#[read_component(AttackStats)]
#[read_component(MovementStats)]
#[write_component(Velocity)]
pub fn ranged_ia_movement(world: &mut SubWorld) {
    let player_position: ArrayVec<(Entity, Position), 4> = {
        let mut query = <(Entity, &Position)>::query()
            .filter(component::<Player>())
            .filter(!component::<Knockback>());
        query.iter(world).map(|(entt, pos)| (*entt, *pos)).collect()
    };

    let mut query = <(
        &mut Velocity,
        &Position,
        &Active,
        &Target,
        &AttackStats,
        &MovementStats,
    )>::query()
    .filter(component::<IA>() & component::<RangedBrain>());

    for (velo, pos, active, target, stats, mov_stats) in query.iter_mut(world) {
        let target_pos = target.0.and_then(|target_entt| {
            player_position
                .iter()
                .find(|player| player.0 == target_entt)
                .map(|player| player.1)
        });

        if !active.0 {
            continue;
        }

        if let Some(p_pos) = target_pos {
            let dx = p_pos.x - pos.x;
            let dy = p_pos.y - pos.y;

            let distance = (dx * dx + dy * dy).sqrt();
            let dir_x = dx / distance;
            let dir_y = dy / distance;

            let tolerance_zone = 50.0;
            let retreat_distance = stats.range - tolerance_zone;

            // TODO: Changer les valeurs hardcodé par mouvement speed
            if distance > stats.range {
                velo.dx = dir_x
                    * mov_stats
                        .accel
                        .clamp(-mov_stats.max_speed, mov_stats.max_speed);
                velo.dy = dir_y
                    * mov_stats
                        .accel
                        .clamp(-mov_stats.max_speed, mov_stats.max_speed);
            } else if distance < retreat_distance {
                velo.dx = -dir_x
                    * mov_stats
                        .accel
                        .clamp(-mov_stats.max_speed, mov_stats.max_speed);
                velo.dy = -dir_y
                    * mov_stats
                        .accel
                        .clamp(-mov_stats.max_speed, mov_stats.max_speed);
            } else {
                velo.dx = 0.0;
                velo.dy = 0.0;
            }
        }
    }
}

#[system(for_each)]
#[filter(component::<KamikazeBrain>() & component::<AttackIntent>())]
pub fn kamikaze_suicide(entt: &Entity, #[resource] damage_queue: &mut Queue<DamageEvent>) {
    damage_queue.data.push(DamageEvent {
        target: *entt,
        amount: 999999, // Montant arbitraire pour OS le kamikaze
    });
}

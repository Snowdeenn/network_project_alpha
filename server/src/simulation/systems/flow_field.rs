use crate::simulation::resources::components::*;
use legion::{Entity, Query, system};
use utils::{
    map::{flow_field::FlowField, grid::Grid},
    math::Vec2,
};

use crate::navigation::FlowFieldManager;

#[system]
#[read_component(Position)]
#[filter(component::<Player>())]
pub fn update_flow_fields(
    world: &legion::world::SubWorld,
    #[resource] grid: &Grid,
    #[resource] flow_field_manager: &mut FlowFieldManager,
    query: &mut Query<(Entity, &Position)>,
) {
    for (player_entt, pos) in query.iter(world) {
        let player_vec = Vec2::new(pos.x as f32, pos.y as f32);

        let field = flow_field_manager
            .fields
            .entry(*player_entt)
            .or_insert_with(|| FlowField::new(grid.width(), grid.height()));
        if field.needs_update(grid, player_vec) {
            field.compute(grid, player_vec);
        }
    }
}

use legion::Entity;
use utils::map::flow_field::FlowField;
use utils::map::grid::Grid;
use utils::math::Vec2;

#[derive(Default)]
pub struct FlowFieldManager {
    pub fields: std::collections::HashMap<Entity, FlowField>,
}

impl FlowFieldManager {
    pub fn get_direction(&self, grid: &Grid, target_entity: Entity, pos: Vec2) -> Vec2 {
        self.fields
            .get(&target_entity)
            .map(|field| field.get_direction(grid, pos))
            .unwrap_or(Vec2::zero())
    }
}
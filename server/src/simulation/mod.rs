pub mod resources;
pub mod systems;

#[inline]
pub fn run_simulation(
    schedule: &mut legion::Schedule,
    world: &mut legion::World,
    resources: &mut legion::Resources,
) {
    schedule.execute(world, resources);
}

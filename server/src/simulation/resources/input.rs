use crate::simulation::resources::components;
use legion::EntityStore;

pub fn process_input(net: &mut crate::net::GameNetServer, resources: &mut legion::Resources, world: &mut legion::world::SubWorld) {
    let mut buff_manager = resources
        .get_mut::<utils::buffer::BufferManager>()
        .expect("[Ressource] devrait retourner le BufferManager");

    let (input_id, inputs) = buff_manager
        .acquire::<Vec<(u64, utils::protocol::InputPacket)>>()
        .expect("[BufferManager] devrait retourner un tuple id data");

    net.drain_inputs_into(inputs);

    for (client_id, packet) in inputs {
        if let Some(entity) = resources
            .get::<crate::session::PlayerRegistry>()
            .unwrap()
            .get_entity(*client_id)
        {
            apply_input(world, entity, packet);
        }
    }
    buff_manager.release(input_id);
}

fn apply_input(
    world: &mut legion::world::SubWorld,
    entity: legion::Entity,
    packet: &utils::protocol::InputPacket,
) {
    if let Ok(mut entry) = world.entry_mut(entity) {
        if let Ok(active) = entry.get_component::<components::Active>() {
            if !active.0 {
                return;
            }
        }
        if let Ok(state) = entry.get_component_mut::<components::InputState>() {
            state.move_dir = packet.move_dir;
            state.aim_dir = packet.aim_dir;
            state.dash = packet.dash;
            state.spell = packet.spell;
            state.attack = packet.attack;
        }
    }
}

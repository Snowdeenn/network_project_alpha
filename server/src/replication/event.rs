use legion::world::Entity;
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: u32,
}

#[derive(Debug)]
pub struct EnemyDied(pub Entity);

#[derive(Debug)]
pub struct CoinEvent {
    pub pos: [f32; 2],
}

pub fn process_game_event(
    net: &mut crate::net::GameNetServer,
    resources: &mut legion::Resources,
    world: &mut legion::world::SubWorld,
) {
    use crate::simulation::resources::components;
    use legion::EntityStore;
    
    let mut game_events = resources
        .get_mut::<crate::utils::Queue<utils::protocol::GameEvent>>()
        .expect("GameEventQueue pas dans les ressources");
    let mapping = resources
        .get::<crate::session::PlayerRegistry>()
        .expect("EntityToClient pas dans les ressources");

    for event in game_events.data.drain(..) {
        match event.kind {
            utils::protocol::GameEventKind::PlayerDied { entity_id } => {
                if let Some(client_id) = mapping.entity_to_client(entity_id) {
                    println!("Envoi de la mort au client concerné : {}", client_id);
                    net.send_event(client_id, &event);

                    if let Some(entity) = resources
                        .get::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .get_entity(client_id)
                    {
                        if let Ok(mut entry) = world.entry_mut(entity) {
                            if let Ok(active) = entry.get_component_mut::<components::Active>() {
                                active.0 = false;
                            }
                            if let Ok(vel) = entry.get_component_mut::<components::Velocity>() {
                                vel.dx = 0.0;
                                vel.dy = 0.0;
                            }
                        }
                    }
                }
            }
            _ => {
                net.broadcast_event(&event);
            }
        }
    }
}

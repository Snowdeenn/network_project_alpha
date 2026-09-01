
use crate::utils::Queue;
use crate::simulation::resources::components::*;
use crate::replication::event::*;
use legion::world::SubWorld;
use legion::*;
use utils::buffer::BufferManager;
use utils::protocol::{GameEvent, GameEventKind};

#[system]
#[write_component(Health)]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(EntityId)]
#[read_component(Active)]
pub fn health(
    world: &mut SubWorld,
    #[resource] enemy_die_queue: &mut Queue<EnemyDied>,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
    #[resource] buff_manager: &mut BufferManager,
) {
    let (deads_id, dead) = buff_manager
        .acquire::<Vec<Entity>>()
        .expect("[BufferManager] devrait retourner un tuple avec l'id et le Vec<Entity>");

    dead.extend(
        <(Entity, &mut Health, &Active)>::query()
            .iter_mut(world)
            .filter(|(_, h, active)| h.hp == 0 && h.state != HealthState::Dead && active.0)
            .map(|(e, h, _)| {
                h.state = HealthState::Dead;
                *e
            }),
    );

    for entity in dead.iter() {
        if let Ok(entry) = world.entry_mut(*entity) {
            if entry.get_component::<IA>().is_ok() {
                enemy_die_queue.data.push(EnemyDied(*entity));
            }
            if entry.get_component::<Player>().is_ok() {
                let id = entry
                    .get_component::<EntityId>()
                    .expect("[Health System] Joueur sans EntityId");
                
                // Toujours envoyer PlayerDied au client concerné
                game_event_queue.data.push(GameEvent {
                    kind: GameEventKind::PlayerDied { entity_id: id.0 },
                });
            }
        }
    }
    buff_manager.release(deads_id);
}

#[system]
#[write_component(Health)]
#[read_component(Player)]
pub fn apply_damage(
    world: &mut SubWorld,
    #[resource] damage_queue: &mut Queue<DamageEvent>,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
) {
    for event in damage_queue.data.iter() {
        if let Ok(mut entry) = world.entry_mut(event.target) {
            if let Ok(health) = entry.get_component_mut::<Health>() {
                health.hp = health.hp.saturating_sub(event.amount);
                // On envoi l'event quand c'est le joueur
                if let Ok(_) = entry.get_component::<Player>() {
                    game_event_queue.push(GameEvent {
                        kind: GameEventKind::PlayerHit,
                    });
                }
            }
        }
    }
}

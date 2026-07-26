use crate::core::config::SharedLives;
use crate::core::queue::Queue;
use crate::core::simulation::components::*;
use crate::core::simulation::event::*;
use legion::world::SubWorld;
use legion::*;
use shared::buffer::BufferManager;
use shared::protocol::{GameEvent, GameEventKind};

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
    #[resource] shared_lives: &mut SharedLives,
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

                shared_lives.remaining = shared_lives.remaining.saturating_sub(1);

                // Toujours envoyer PlayerDied au client concerné
                game_event_queue.data.push(GameEvent {
                    kind: GameEventKind::PlayerDied { entity_id: id.0 },
                });

                // Broadcast vies restantes
                game_event_queue.data.push(GameEvent {
                    kind: GameEventKind::SharedLivesUpdate {
                        remaining: shared_lives.remaining,
                        max: shared_lives.max,
                    },
                });

                // Game over si plus de vies
                if shared_lives.remaining == 0 {
                    game_event_queue.data.push(GameEvent {
                        kind: GameEventKind::GameOver,
                    });
                }
            }
        }
    }
    buff_manager.release(deads_id);
}

#[system]
#[write_component(Health)]
pub fn apply_damage(world: &mut SubWorld, #[resource] damage_queue: &mut Queue<DamageEvent>) {
    for event in damage_queue.data.iter() {
        if let Ok(mut entry) = world.entry_mut(event.target) {
            if let Ok(health) = entry.get_component_mut::<Health>() {
                health.hp = health.hp.saturating_sub(event.amount);
            }
        }
    }
}

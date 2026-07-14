use legion::*;
use legion::world::SubWorld;
use crate::simulation::components::*;
use crate::simulation::event::*;
use shared::protocol::{GameEvent, GameEventKind};
use crate::config::SharedLives;

#[system]
#[write_component(Health)]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(EntityId)]
#[read_component(Active)]
pub fn health(
    world: &mut SubWorld,
    #[resource] enemy_die_queue: &mut EnemyDiedQueue,
    #[resource] game_event_queue: &mut GameEventQueue,
    #[resource] shared_lives: &mut SharedLives,
) {
    let dead: Vec<Entity> = <(Entity, &mut Health, &Active)>::query()
        .iter_mut(world)
        .filter(|(_, h, active)| h.hp == 0 && h.state != HealthState::Dead && active.0)
        .map(|(e, h, _)| {
            h.state = HealthState::Dead;
            *e
        })
        .collect();

    for entity in dead {
        if let Ok(entry) = world.entry_mut(entity) {
            if entry.get_component::<IA>().is_ok() {
                enemy_die_queue.0.push(EnemyDied(entity));
            }
            if entry.get_component::<Player>().is_ok() {
                let id = entry
                    .get_component::<EntityId>()
                    .expect("[Health System] Joueur sans EntityId");

                shared_lives.remaining = shared_lives.remaining.saturating_sub(1);

                // Toujours envoyer PlayerDied au client concerné
                game_event_queue.0.push(GameEvent {
                    kind: GameEventKind::PlayerDied { entity_id: id.0 },
                });

                // Broadcast vies restantes
                game_event_queue.0.push(GameEvent {
                    kind: GameEventKind::SharedLivesUpdate {
                        remaining: shared_lives.remaining,
                        max: shared_lives.max,
                    },
                });

                // Game over si plus de vies
                if shared_lives.remaining == 0 {
                    game_event_queue.0.push(GameEvent {
                        kind: GameEventKind::GameOver,
                    });
                }
            }
        }
    }
}

#[system]
#[write_component(Health)]
pub fn apply_damage(world: &mut SubWorld, #[resource] damage_queue: &mut DamageQueue) {
    for event in damage_queue.0.iter() {
        if let Ok(mut entry) = world.entry_mut(event.target) {
            if let Ok(health) = entry.get_component_mut::<Health>() {
                health.hp = health.hp.saturating_sub(event.amount);
            }
        }
    }
}
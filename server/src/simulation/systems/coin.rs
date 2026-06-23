use legion::*;
use legion::world::SubWorld;
use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::eco::*;
use crate::simulation::helper::aabb_overlap;
use std::collections::HashMap;

#[system]
#[read_component(Position)]
#[read_component(IA)]
pub fn coin_push_to_queue(
    word: &mut SubWorld,
    #[resource] enemy_die_queue: &EnemyDiedQueue,
    #[resource] coin_spawn_queue: &mut CoinSpawnQueue,
) {
    for event in enemy_die_queue.0.iter() {
        if let Ok(entry) = word.entry_ref(event.0) {
            if entry.get_component::<IA>().is_ok() {
                coin_spawn_queue.0.push(CoinEvent {
                    pos: [
                        entry
                            .get_component::<Position>()
                            .map(|p| p.x as f32)
                            .unwrap_or_default(),
                        entry
                            .get_component::<Position>()
                            .map(|p| p.y as f32)
                            .unwrap_or_default(),
                    ],
                });
            }
        }
    }
}

#[system]
#[write_component(Active)]
#[write_component(Position)]
#[write_component(CoinValue)]
pub fn coin_spawn(
    world: &mut SubWorld,
    #[resource] coin_spawn_queue: &mut CoinSpawnQueue,
    #[resource] coin_pool: &CoinPool,
) {
    for coin in coin_pool.coins.iter() {
        if let Ok(mut entry) = world.entry_mut(*coin) {
            if let Ok(active) = entry.get_component_mut::<Active>() {
                if active.0 {
                    continue; // Skip coins actifs
                } else {
                    if let Some(event) = coin_spawn_queue.0.pop() {
                        *active = Active(true);
                        if let Ok(pos) = entry.get_component_mut::<Position>() {
                            pos.x = event.pos[0] as f64;
                            pos.y = event.pos[1] as f64;
                        }
                    }
                }
            }
        }
    }
}

#[system]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(Coin)]
pub fn coin_pickup(word: &mut SubWorld, #[resource] pick_up_queue: &mut PickupQueue) {
    let players: std::collections::HashSet<Entity> = <Entity>::query()
        .filter(component::<Player>())
        .iter(word)
        .copied()
        .collect();

    let coins: std::collections::HashSet<Entity> = <Entity>::query()
        .filter(component::<Coin>())
        .iter(word)
        .copied()
        .collect();

    let mut query = <(Entity, &Position, &Collider, &Active)>::query();
    let entities: Vec<_> = query
        .iter(word)
        .filter(|(_, _, _, active)| active.0)
        .map(|(e, p, c, _)| (*e, *p, *c))
        .collect();

    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ent_a, pos_a, col_a) = entities[i];
            let (ent_b, pos_b, col_b) = entities[j];

            if let Some(_) = aabb_overlap(&pos_a, &col_a, &pos_b, &col_b) {
                let a_is_player = players.contains(&ent_a);
                let b_is_player = players.contains(&ent_b);
                let a_is_coin = coins.contains(&ent_a);
                let b_is_coin = coins.contains(&ent_b);

                if a_is_player && b_is_coin {
                    pick_up_queue.0.push((ent_a, ent_b));
                }

                if b_is_player && a_is_coin {
                    pick_up_queue.0.push((ent_b, ent_a));
                }
            }
        }
    }
}
#[system]
#[read_component(CoinValue)]
#[write_component(Active)]
pub fn apply_pickup(
    world: &mut SubWorld,
    #[resource] pick_up_queue: &mut PickupQueue,
    #[resource] gold: &mut PlayerGold,
    #[resource] players_entities: &HashMap<u64, Entity>,
) {
    for (player_entity, coin) in pick_up_queue.0.iter() {
        if let Ok(mut entry) = world.entry_mut(*coin) {
            if let Ok(active) = entry.get_component_mut::<Active>() {
                *active = Active(false);
            }

            if let Ok(value) = entry.get_component::<CoinValue>() {
                let player_id = players_entities
                    .iter()
                    .find(|&(_, &ent)| ent == *player_entity)
                    .map(|(&id, _)| id);

                if let Some(id) = player_id {
                    gold.add(id, value.0);

                    println!(
                        "Le joueur {} a ramassé une pièce d'une valeur de {} !",
                        id, value.0
                    );
                }
            }
        }
    }
}
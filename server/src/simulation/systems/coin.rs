use crate::player_registry::PlayerRegistry;
use crate::pool::GamePools;
use crate::pool::PoolManager;
use crate::queue::Queue;
use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::helper::aabb_overlap;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::ids::CoinTag;

#[system]
#[read_component(Position)]
#[read_component(IA)]
pub fn coin_push_to_queue(
    word: &mut SubWorld,
    #[resource] enemy_die_queue: &Queue<EnemyDied>,
    #[resource] coin_spawn_queue: &mut Queue<CoinEvent>,
) {
    for event in enemy_die_queue.data.iter() {
        if let Ok(entry) = word.entry_ref(event.0) {
            if entry.get_component::<IA>().is_ok() {
                coin_spawn_queue.data.push(CoinEvent {
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
    command: &mut CommandBuffer,
    #[resource] coin_spawn_queue: &mut Queue<CoinEvent>,
    #[resource] pool_manager: &mut PoolManager<GamePools>,
) {
    while let Some(event) = coin_spawn_queue.data.pop() {
        if let Some((id, entity)) = pool_manager.acquire::<CoinTag>(world) {
            if let Ok(mut entry) = world.entry_mut(entity) {
                if let Ok(pos) = entry.get_component_mut::<Position>() {
                    pos.x = event.pos[0] as f64;
                    pos.y = event.pos[1] as f64;
                }
                if let Ok(value) = entry.get_component_mut::<CoinValue>() {
                    value.0 = rand::random::<u32>() % 10 + 1;
                }
            }
            command.add_component(entity, PoolId(id));
        } else {
            eprintln!("[coin_spawn] Pool de coins plein");
            break;
        }
    }
}

#[system]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(Coin)]
pub fn coin_pickup(word: &mut SubWorld, #[resource] pick_up_queue: &mut Queue<(Entity, Entity)>) {
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
                    pick_up_queue.data.push((ent_a, ent_b));
                }

                if b_is_player && a_is_coin {
                    pick_up_queue.data.push((ent_b, ent_a));
                }
            }
        }
    }
}
#[system]
#[read_component(CoinValue)]
#[read_component(PoolId<CoinTag>)]
#[read_component(EntityId)]
#[write_component(Active)]
pub fn apply_pickup(
    world: &mut SubWorld,
    #[resource] pick_up_queue: &mut Queue<(Entity, Entity)>,
    #[resource] registry: &mut PlayerRegistry,
    #[resource] pool_manager: &mut PoolManager<GamePools>,
) {
    for (player_entity, coin) in pick_up_queue.data.iter() {
        let pool_id = world
            .entry_ref(*coin)
            .ok()
            .and_then(|entry| entry.get_component::<PoolId<CoinTag>>().ok().map(|p| p.0));

        let gold_amount = world
            .entry_ref(*coin)
            .ok()
            .and_then(|e| e.get_component::<CoinValue>().ok().map(|v| v.0));

        let client_id = world
            .entry_ref(*player_entity)
            .ok()
            .and_then(|e| e.get_component::<EntityId>().ok().map(|id| id.0))
            .and_then(|id| registry.entity_to_client(id));

        if let (Some(amount), Some(client_id)) = (gold_amount, client_id) {
            registry.add_gold(client_id, amount);
            println!(
                "Le joueur {} a ramassé une pièce de {} !",
                client_id, amount
            );
        }

        if let Some(id) = pool_id {
            pool_manager.release(id, world);
        }
    }
}

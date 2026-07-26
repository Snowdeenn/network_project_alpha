use crate::core::player_registry::PlayerRegistry;
use crate::core::pool::GamePools;
use crate::core::pool::PoolManager;
use crate::core::queue::Queue;
use crate::core::simulation::components::*;
use crate::core::simulation::event::*;
use crate::core::simulation::helper::aabb_overlap;
use crate::core::simulation::spatial_grid::SpatialGrid;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::buffer::BufferManager;
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
pub fn coin_pickup(
    word: &mut SubWorld,
    #[resource] pick_up_queue: &mut Queue<(Entity, Entity)>,
    #[resource] buff_manager: &mut BufferManager,
    #[resource] grid: &mut SpatialGrid,
) {
    grid.clear();

    let coins_id = buff_manager.acquire_id::<Vec<(Entity, Position, Collider)>>();
    let players_id = buff_manager.acquire_id::<Vec<(Entity, Position, Collider)>>();
    let candidates_id = buff_manager.acquire_id::<Vec<usize>>();

    {
        let coins = buff_manager
            .get_mut::<Vec<(Entity, Position, Collider)>>(coins_id)
            .expect("[Buffer Manager] Vec<Coins> introuvable");

        coins.extend(
            <(Entity, &Position, &Collider, &Active)>::query()
                .filter(component::<Coin>())
                .iter(word)
                .filter(|(_, _, _, a)| a.0)
                .map(|(e, p, c, _)| (*e, *p, *c)),
        );

        let coins = buff_manager
            .get::<Vec<(Entity, Position, Collider)>>(coins_id)
            .expect("[Buffer Manager] Coins introuvable");

        // On verifie si il y a des coins actifs sur la map
        // Si il n'y en a pas on early return pour éviter de griller des cylces cpu pour rien
        if coins.is_empty() {
            buff_manager.release(coins_id);
            buff_manager.release(players_id);
            buff_manager.release(candidates_id);
            return;
        }
        for (idx, (_, pos, col)) in coins.iter().enumerate() {
            grid.insert(idx, pos, col);
        }
    }
    {
        let players = buff_manager
            .get_mut::<Vec<(Entity, Position, Collider)>>(players_id)
            .expect("[Buffer Manager] Vec<Players> introuvable");

        players.extend(
            <(Entity, &Position, &Collider)>::query()
                .filter(component::<Player>())
                .iter(word)
                .map(|(e, p, c)| (*e, *p, *c)),
        );
    }

    grid.build();
    let mut candidates = std::mem::take(
        buff_manager
            .get_mut::<Vec<usize>>(candidates_id)
            .expect("[Buffer Manager] Candidates introuvable"),
    );

    let coins = buff_manager
        .get::<Vec<(Entity, Position, Collider)>>(coins_id)
        .expect("[Buffer Manager] Coins introuvable");
    let players = buff_manager
        .get::<Vec<(Entity, Position, Collider)>>(players_id)
        .expect("[Buffer Manager] Players introuvable");

    for (player_entity, player_pos, player_col) in players.iter() {
        grid.query(player_pos, player_col, &mut candidates);

        candidates.sort_unstable();
        candidates.dedup();

        for &coin_idx in candidates.iter() {
            let (coin_entity, coin_pos, coin_col) = &coins[coin_idx];

            if aabb_overlap(player_pos, player_col, coin_pos, coin_col).is_some() {
                pick_up_queue.data.push((*player_entity, *coin_entity));
            }
        }
    }

    buff_manager.release(coins_id);
    buff_manager.release(players_id);
    buff_manager.release(candidates_id);
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

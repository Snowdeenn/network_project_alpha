use crate::simulation::resources::components::Coin;
use crate::session::player_registry::PlayerRegistry;
use crate::simulation::resources::components::{Active, EntityId, Health, IA, Player, Position, Projectile};
use crate::simulation::resources::wave::{WaveManager, WaveState as SimWaveState};
use legion::*;
use utils::protocol::{EntityKind, EntityState, PlayerInfo, WaveInfo, WaveState};

pub fn build_wave_info(resources: &Resources) -> WaveInfo {
    resources
        .get::<WaveManager>()
        .map(|wm| WaveInfo {
            wave_number: wm.current_wave as u32,
            enemy_remaining: wm.enemies_remaining,
            wave_state: match wm.wave_state {
                SimWaveState::InProgress => WaveState::InProgress,
                SimWaveState::BetweenWave(d) => WaveState::BetweenWave {
                    remaining_ms: d.as_millis() as u32,
                },
                SimWaveState::Waiting => WaveState::Waiting,
            },
        })
        .unwrap_or(WaveInfo {
            wave_number: 0,
            enemy_remaining: 0,
            wave_state: WaveState::InProgress,
        })
}

pub fn build_entities(world: &World, entities: &mut Vec<EntityState>) {
    // joueurs
    {
        let mut player_query = <(&EntityId, &Position, &Health, &Active)>::query()
            .filter(component::<Player>() & component::<Active>());
        for (id, pos, health, active) in player_query.iter(world) {
            if !active.0 {
                continue;
            }
            entities.push(EntityState {
                entity_id: id.0,
                position: [pos.x as f32, pos.y as f32],
                health: health.hp as f32,
                max_health: 100.0,
                entity_kind: EntityKind::Player,
            });
        }
    }

    // ennemis
    {
        let mut ia_query = <(&EntityId, &Position, &Health, &Active)>::query()
            .filter(component::<IA>() & component::<Active>());
        for (id, pos, health, active) in ia_query.iter(world) {
            if !active.0 {
                continue;
            }
            entities.push(EntityState {
                entity_id: id.0,
                position: [pos.x as f32, pos.y as f32],
                health: health.hp as f32,
                max_health: 100.0,
                entity_kind: EntityKind::Enemy,
            });
        }
    }

    // coins
    {
        let mut coin_query = <(&EntityId, &Position, &Active)>::query()
            .filter(component::<Coin>() & component::<Active>());
        for (id, pos, active) in coin_query.iter(world) {
            if !active.0 {
                continue;
            }
            entities.push(EntityState {
                entity_id: id.0,
                position: [pos.x as f32, pos.y as f32],
                health: 0.0,
                max_health: 0.0,
                entity_kind: EntityKind::Coin,
            });
        }
    }

    // projectiles
    {
        let mut proj_query = <(&EntityId, &Position, &Active)>::query()
            .filter(component::<Projectile>() & component::<Active>());
        for (id, pos, active) in proj_query.iter(world) {
            if !active.0 {
                continue;
            }
            entities.push(EntityState {
                entity_id: id.0,
                position: [pos.x as f32, pos.y as f32],
                health: 0.0,
                max_health: 0.0,
                entity_kind: EntityKind::Projectile,
            });
        }
    }
}

pub fn build_player_info(
    player_id: u64,
    world: &mut World,
    resources: &Resources,
) -> Option<PlayerInfo> {
    let registry = resources.get::<PlayerRegistry>().unwrap();
    let entity = registry.get_entity(player_id)?;
    let gold = registry.get_gold(player_id);
    drop(registry);

    let entry = world.entry(entity)?;
    let health = entry.get_component::<Health>().ok()?;

    Some(PlayerInfo {
        health: health.hp as f32,
        max_health: 100.0,
        gold,
    })
}

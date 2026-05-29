use crate::simulation::components::{Active, EntityId, Health, IA, Player, Position};
use crate::simulation::wave::{WaveManager, WaveState as SimWaveState};
use legion::*;
use shared::protocol::{EntityKind, EntityState, StateSnapshot, WaveInfo, WaveState, PlayerInfo};
use crate::Coin;
use crate::Gold;

pub fn build_snapshot(world: &World, resources: &Resources, tick_id: u64) -> StateSnapshot {
    let wave_info = build_wave_info(resources);
    let entities = build_entities(world);
    StateSnapshot {
        tick_id,
        entities,
        wave_info,
        player_info: build_player_info(world, resources),
    }
}

fn build_wave_info(resources: &Resources) -> WaveInfo {
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
            },
        })
        .unwrap_or(WaveInfo {
            wave_number: 0,
            enemy_remaining: 0,
            wave_state: WaveState::InProgress,
        })
}

fn build_entities(world: &World) -> Vec<EntityState> {
    let mut entities = Vec::new();

    // joueurs
    {
        let mut player_query = <(&EntityId, &Position, &Health)>::query()
            .filter(component::<Player>() & component::<Active>());
        for (id, pos, health) in player_query.iter(world) {
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
        let mut ia_query = <(&EntityId, &Position, &Health)>::query()
            .filter(component::<IA>() & component::<Active>());
        for (id, pos, health) in ia_query.iter(world) {
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
        let mut coin_query =
            <(&EntityId, &Position)>::query().filter(component::<Coin>() & component::<Active>());
        for (id, pos) in coin_query.iter(world) {
            entities.push(EntityState {
                entity_id: id.0,
                position: [pos.x as f32, pos.y as f32],
                health: 0.0,
                max_health: 0.0,
                entity_kind: EntityKind::Coin,
            });
        }
    }

    entities
}

fn build_player_info(world: &World, resources: &Resources) -> Option<PlayerInfo> {
    let gold = resources.get::<Gold>()?.0;

    let mut query = <(&Health,)>::query().filter(component::<Player>() & component::<Active>());
    let (health,) = query.iter(world).next()?;

    Some(PlayerInfo {
        health:     health.hp as f32,
        max_health: 100.0,
        gold,
    })
}


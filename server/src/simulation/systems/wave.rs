use crate::pool::GamePools;
use crate::pool::PoolManager;
use crate::queue::Queue;
use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::wave::*;
use crate::simulation::systems::spawn;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::ids::EnemyTag;
use shared::protocol::{GameEvent, GameEventKind};
use std::str::FromStr;
use std::time::Duration;

#[system]
#[write_component(Active)]
#[read_component(PoolId<EnemyTag>)]
pub fn wave_death_reaper(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] wave_manager: &mut WaveManager,
    #[resource] enemy_die_queue: &mut Queue<EnemyDied>,
    #[resource] pool_manager: &mut PoolManager<GamePools>,
) {
    for event in enemy_die_queue.data.iter() {
        wave_manager.enemies_remaining = wave_manager.enemies_remaining.saturating_sub(1);
        let pool_id = world
            .entry_ref(event.0)
            .ok()
            .and_then(|entry| entry.get_component::<PoolId<EnemyTag>>().ok().map(|p| p.0));

        if let Some(id) = pool_id {
            pool_manager.release(id, world);
        }
        command.remove_component::<RangedBrain>(event.0);
        command.remove_component::<MeleeBrain>(event.0);
        command.remove_component::<KamikazeBrain>(event.0);
    }
}

#[system]
#[write_component(Health)]
#[write_component(Active)]
#[write_component(Position)]
#[write_component(EntityId)]
#[write_component(Target)]
#[write_component(AttackStats)]
#[write_component(MovementStats)]
pub fn wave_spawner(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] wave_manager: &mut WaveManager,
    #[resource] dt: &Duration,
    #[resource] wave_configs: &WaveConfigs,
    #[resource] pool_manager: &mut PoolManager<GamePools>,
    #[resource] enemy_config: &EnemyConfigs,
) {
    if let WaveState::InProgress = wave_manager.wave_state {
        wave_manager.spawn_timer = wave_manager.spawn_timer.saturating_sub(*dt);

        if wave_manager.spawn_timer.is_zero() && wave_manager.enemies_to_spawn > 0 {
            if let Some((id, entity)) = pool_manager.acquire::<EnemyTag>(world) {
                let current_wave_config = &wave_configs.0[wave_manager.current_wave];
                let base_hp = current_wave_config.enemy_hp;
                let base_speed = current_wave_config.enemy_speed;

                let enemy_type = pick_enemy_type(current_wave_config);

                if let Some(config) = enemy_config.0.get(enemy_type.to_str()) {
                    if let Ok(mut entry) = world.entry_mut(entity) {
                        spawn::configure_enemy(&mut entry, config, base_hp, base_speed);

                        match enemy_type {
                            EnemyType::Melee => command.add_component(entity, MeleeBrain),
                            EnemyType::Ranged => command.add_component(entity, RangedBrain),
                            EnemyType::Kamikaze => command.add_component(entity, KamikazeBrain),
                        }
                        command.add_component(entity, PoolId(id));
                    }
                }

                wave_manager.spawn_timer =
                    Duration::from_millis(current_wave_config.spawn_interval_ms);
                wave_manager.enemies_to_spawn -= 1;
            }
        } else {
            eprintln!("[Wave Spawner] pool vide");
        }
    }
}

#[system]
pub fn wave_flow_manager(
    #[resource] wave_manager: &mut WaveManager,
    #[resource] dt: &Duration,
    #[resource] wave_configs: &WaveConfigs,
    #[resource] game_event_queue: &mut Queue<GameEvent>,
) {
    match wave_manager.wave_state {
        WaveState::InProgress => {
            if wave_manager.enemies_remaining == 0 && wave_manager.enemies_to_spawn == 0 {
                wave_manager.wave_state = WaveState::BetweenWave(Duration::from_secs(20));
                game_event_queue.data.push(GameEvent {
                    kind: GameEventKind::WaveEnd {
                        time_between_wave: Duration::from_secs(20),
                    },
                });
            }
        }
        WaveState::BetweenWave(d) => {
            let remaining = d.saturating_sub(*dt);
            if remaining.is_zero() {
                wave_manager.current_wave += 1;

                if let Some(config) = wave_configs.0.get(wave_manager.current_wave) {
                    wave_manager.enemies_to_spawn = config.enemy_count;
                    wave_manager.enemies_remaining = config.enemy_count;
                    wave_manager.spawn_timer = Duration::from_millis(config.spawn_interval_ms);
                    wave_manager.wave_state = WaveState::InProgress;

                    game_event_queue.data.push(GameEvent {
                        kind: GameEventKind::WaveStart {
                            wave_number: wave_manager.current_wave as u32,
                            enemy_count: config.enemy_count,
                            enemy_hp: config.enemy_hp,
                            enemy_speed: config.enemy_speed as f32,
                        },
                    });
                } else {
                    // Fin du jeu (plus de vagues définies)
                    wave_manager.wave_state = WaveState::BetweenWave(Duration::ZERO);
                }
            } else {
                wave_manager.wave_state = WaveState::BetweenWave(remaining);
            }
        }
        _ => {}
    }
}

pub fn pick_enemy_type(config: &WaveConfig) -> EnemyType {
    let total_weight: f64 = config.enemy_weights.values().sum();
    if total_weight <= 0.0 {
        return EnemyType::Melee;
    }
    let mut rng_weight = rand::random::<f64>() * total_weight;
    for (etype, weight) in &config.enemy_weights {
        if rng_weight <= *weight {
            return EnemyType::from_str(etype).unwrap_or(EnemyType::Melee);
        }
        rng_weight -= *weight;
    }
    EnemyType::Melee
}

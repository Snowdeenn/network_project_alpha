use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::helper::PlayerPos;
use crate::simulation::wave::*;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::protocol::{GameEvent, GameEventKind};
use std::str::FromStr;
use std::time::Duration;

#[system]
#[write_component(Active)]
pub fn wave_death_reaper(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] wave_manager: &mut WaveManager,
    #[resource] enemy_die_queue: &mut EnemyDiedQueue,
) {
    for event in enemy_die_queue.0.iter() {
        wave_manager.enemies_remaining = wave_manager.enemies_remaining.saturating_sub(1);
        if let Ok(mut entry) = world.entry_mut(event.0) {
            if let Ok(active) = entry.get_component_mut::<Active>() {
                active.0 = false;
            }
        }
        command.remove_component::<RangedBrain>(event.0);
        command.remove_component::<MeleeBrain>(event.0);
        command.remove_component::<KamikazeBrain>(event.0);
    }
}

const SPAWN_RADIUS: f64 = 800.0;
use std::f64::consts::PI;

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
    #[resource] player_pos: &PlayerPos,
    #[resource] enemy_pool: &EnemyPool,
    #[resource] enemy_config: &EnemyConfigs,
) {
    if let WaveState::InProgress = wave_manager.wave_state {
        wave_manager.spawn_timer = wave_manager.spawn_timer.saturating_sub(*dt);

        if wave_manager.spawn_timer.is_zero() && wave_manager.enemies_to_spawn > 0 {
            for entity in enemy_pool.pool.iter() {
                if let Ok(mut entry) = world.entry_mut(*entity) {
                    if let Ok(active) = entry.get_component_mut::<Active>() {
                        if active.0 {
                            continue;
                        } // Déjà actif, on passe au suivant

                        *active = Active(true);

                        if let Ok(id) = entry.get_component_mut::<EntityId>() {
                            *id = EntityId(crate::next_id());
                        }

                        if let Ok(pos) = entry.get_component_mut::<Position>() {
                            let angle = rand::random::<f64>() * 2.0 * PI;
                            pos.x = player_pos.x + angle.cos() * SPAWN_RADIUS;
                            pos.y = player_pos.y + angle.sin() * SPAWN_RADIUS;
                        }

                        if let Ok(target) = entry.get_component_mut::<Target>() {
                            target.0 = None;
                        }

                        let current_wave_config = &wave_configs.0[wave_manager.current_wave];
                        let base_hp = current_wave_config.enemy_hp;
                        let base_speed = current_wave_config.enemy_speed;

                        let mut enemy_type = EnemyType::Melee;
                        let total_weight: f64 = current_wave_config.enemy_weights.values().sum();

                        if total_weight > 0.0 {
                            let mut rng_weight = rand::random::<f64>() * total_weight;

                            for (etype, weight) in &current_wave_config.enemy_weights {
                                if rng_weight <= *weight {
                                    enemy_type = EnemyType::from_str(etype).unwrap();
                                    break;
                                }
                                rng_weight -= *weight;
                            }
                        }

                        if let Some(config) = enemy_config.0.get(enemy_type.to_str()) {
                            if let Ok(health) = entry.get_component_mut::<Health>() {
                                health.hp = (base_hp as f64 * config.hp_modifier) as u32;
                                health.max_hp = (base_hp as f64 * config.hp_modifier) as u32;
                                health.state = HealthState::Alive;
                            }

                            if let Ok(speed) = entry.get_component_mut::<MovementStats>() {
                                speed.accel = base_speed * config.speed_modifier;
                                speed.max_speed = config.max_speed;
                            }

                            if let Ok(attack_stats) = entry.get_component_mut::<AttackStats>() {
                                attack_stats.range = config.range;
                                attack_stats.damage = config.damage;
                                attack_stats.projectile_speed = config.projectile_speed;
                                attack_stats.box_half_length = config.box_half_length;
                                attack_stats.box_half_width = config.box_half_width;
                            }

                            match enemy_type {
                                EnemyType::Melee => command.add_component(*entity, MeleeBrain),
                                EnemyType::Ranged => command.add_component(*entity, RangedBrain),
                                EnemyType::Kamikaze => command.add_component(*entity, KamikazeBrain),
                            }
                        }

                        // Relancer le chrono de spawn
                        wave_manager.spawn_timer =
                            Duration::from_millis(current_wave_config.spawn_interval_ms);
                        wave_manager.enemies_to_spawn -= 1;
                        break; // On en spawn un seul par frame maximum
                    }
                }
            }
        }
    }
}

#[system]
pub fn wave_flow_manager(
    #[resource] wave_manager: &mut WaveManager,
    #[resource] dt: &Duration,
    #[resource] wave_configs: &WaveConfigs,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    match wave_manager.wave_state {
        WaveState::InProgress => {
            if wave_manager.enemies_remaining == 0 && wave_manager.enemies_to_spawn == 0 {
                wave_manager.wave_state = WaveState::BetweenWave(Duration::from_secs(20));
                game_event_queue.0.push(GameEvent {
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

                    game_event_queue.0.push(GameEvent {
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
    }
}

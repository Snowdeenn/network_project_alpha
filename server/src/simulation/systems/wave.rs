use legion::systems::CommandBuffer;
use legion::*;
use legion::world::SubWorld;
use std::time::Duration;
use crate::simulation::wave::*;
use crate::simulation::components::*;
use crate::simulation::event::*;
use crate::simulation::helper::PlayerPos;
use shared::protocol::{GameEvent, GameEventKind};


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
pub fn wave_spawner(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] wave_manager: &mut WaveManager,
    #[resource] dt: &Duration,
    #[resource] wave_configs: &WaveConfigs,
    #[resource] player_pos: &PlayerPos,
    #[resource] enemy_pool: &EnemyPool,
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

                        if let Ok(health) = entry.get_component_mut::<Health>() {
                            health.hp = wave_configs.0[wave_manager.current_wave].enemy_hp;
                            health.state = HealthState::Alive;
                        }

                        if let Ok(target) = entry.get_component_mut::<Target>() {
                            target.0 = None;
                        }
                        

                        // Ranged IA*
                        // TODO: Ajuster le spawn rate
                        if rand::random::<f64>() > 0.30 {
                            println!("Un rangedbrain a spawn");
                            command.add_component(*entity, RangedBrain);
                            if let Ok(attack_stats) = entry.get_component_mut::<AttackStats>() {
                                attack_stats.range = 300.0;
                                attack_stats.damage = 10;
                                attack_stats.projectile_speed = Some(400.0);
                                attack_stats.box_half_length = 5.0;
                                attack_stats.box_half_width = 5.0;
                            }
                        } else {
                            println!("Un meleebrain a spawn");
                            command.add_component(*entity, MeleeBrain);
                            
                            if let Ok(attack_stats) = entry.get_component_mut::<AttackStats>() {
                                attack_stats.range = 55.0;
                                attack_stats.damage = 15;
                                attack_stats.projectile_speed = None;
                                attack_stats.box_half_length = 20.0;
                                attack_stats.box_half_width = 20.0;
                            }
                        }

                        // Relancer le chrono de spawn
                        wave_manager.spawn_timer = Duration::from_millis(
                            wave_configs.0[wave_manager.current_wave].spawn_interval,
                        );
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
                    wave_manager.spawn_timer = Duration::from_millis(config.spawn_interval);
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
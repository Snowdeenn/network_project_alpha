use std::time::Duration;

use crate::simulation::components::*;
use crate::simulation::eco::{CoinPool, CoinSpawnQueue, Gold, PickupQueue};
use crate::simulation::event::{CoinEvent, DamageEvent, DamageQueue, EnemyDied, EnemyDiedQueue};
use crate::simulation::helper::*;
use crate::simulation::input::*;
use crate::simulation::wave::{EnemyPool, WaveConfigs, WaveManager, WaveState};
use legion::world::SubWorld;
use legion::*;

const ACCEL: f64 = 1500.0;
// todo: Ajouter plusieurs friction en fonction
// du milieu dans lequel le joueur ce déplace.

const FRICTION: f64 = 0.85;
const ARENA_W: f64 = 1920.0;
const ARENA_H: f64 = 1080.0;

#[system(for_each)]
pub fn update_position(pos: &mut Position, velo: &Velocity, #[resource] dt: &Duration) {
    pos.x += velo.dx * (*dt).as_secs_f64();
    pos.y += velo.dy * (*dt).as_secs_f64();
}

#[system(for_each)]
#[filter(component::<Player>())]
pub fn update_player_pos(pos: &Position, #[resource] player_pos: &mut PlayerPos) {
    player_pos.x = pos.x;
    player_pos.y = pos.y;
}

#[system(for_each)]
#[filter(component::<Player>())]
pub fn update_velocity(
    velo: &mut Velocity,
    #[resource] state: &InputState,
    #[resource] dt: &Duration,
) {
    let input_x = state.move_dir[0] as f64 * ACCEL * (*dt).as_secs_f64();
    let input_y = state.move_dir[1] as f64 * ACCEL * (*dt).as_secs_f64();

    velo.dx += input_x;
    velo.dy += input_y;
}

#[system(for_each)]
#[filter(component::<Player>())]
pub fn friction(velo: &mut Velocity) {
    velo.dx *= FRICTION;
    velo.dy *= FRICTION;
}

#[system(for_each)]
#[filter(component::<Player>())]
pub fn dash(
    velo: &mut Velocity,
    dash: &mut Dash,
    #[resource] state: &InputState,
    #[resource] delta_time: &Duration,
) {
    let new_state = match dash.0 {
        DashState::Idle => {
            if state.dash {
                velo.dx *= 7.0;
                velo.dy *= 7.0;
                DashState::Dashing(Duration::from_millis(25))
            } else {
                DashState::Idle
            }
        }

        DashState::Dashing(d) => {
            let remaining = d.saturating_sub(*delta_time);
            if remaining.is_zero() {
                DashState::Cooldown(Duration::from_secs(2))
            } else {
                DashState::Dashing(remaining)
            }
        }

        DashState::Cooldown(d) => {
            let remaining = d.saturating_sub(*delta_time);
            if remaining.is_zero() {
                DashState::Idle
            } else {
                DashState::Cooldown(remaining)
            }
        }
    };
    dash.0 = new_state;
}

#[system(for_each)]
pub fn collide_arena(pos: &mut Position, col: &Collider) {
    pos.x = pos.x.clamp(0.0, ARENA_W - col.w);
    pos.y = pos.y.clamp(0.0, ARENA_H - col.h);
}

#[system]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(IA)]
#[write_component(Velocity)]
#[write_component(Position)]
#[read_component(Active)]
#[read_component(Coin)]
pub fn collide(world: &mut SubWorld, #[resource] damage_queue: &mut DamageQueue) {
    let mut query = <(Entity, &Position, &Collider, &Active)>::query();
    let entities: Vec<_> = query
        .iter(world)
        .filter(|(_, _, _, active)| active.0)
        .filter(|(e, _, _, _)| {
            world
                .entry_ref(**e)
                .map_or(false, |e| !e.get_component::<Coin>().is_ok())
        })
        .map(|(e, p, c, _)| (e, p, c))
        .collect();

    let mut to_resolve: Vec<Resolution> = Vec::new();
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ent_a, pos_a, col_a) = entities[i];
            let (ent_b, pos_b, col_b) = entities[j];

            if let Some((overlap_x, overlap_y)) = aabb_overlap(pos_a, col_a, pos_b, col_b) {
                let center_a_x = pos_a.x + col_a.w / 2.0;
                let center_b_x = pos_b.x + col_b.w / 2.0;
                let center_a_y = pos_a.y + col_a.h / 2.0;
                let center_b_y = pos_b.y + col_b.h / 2.0;

                to_resolve.push(Resolution {
                    ent_a: *ent_a,
                    ent_b: *ent_b,
                    overlap_x,
                    overlap_y,
                    dir_x: (center_a_x - center_b_x).signum(),
                    dir_y: (center_a_y - center_b_y).signum(),
                    axis: overlap_x < overlap_y,
                });
                let a_is_player = {
                    world
                        .entry_ref(*ent_a)
                        .map(|e| e.get_component::<Player>().is_ok())
                        .unwrap_or(false)
                };

                let a_is_ia = {
                    world
                        .entry_ref(*ent_a)
                        .map(|e| e.get_component::<IA>().is_ok())
                        .unwrap_or(false)
                };

                let b_is_player = {
                    world
                        .entry_ref(*ent_b)
                        .map(|e| e.get_component::<Player>().is_ok())
                        .unwrap_or(false)
                };

                let b_is_ia = {
                    world
                        .entry_ref(*ent_b)
                        .map(|e| e.get_component::<IA>().is_ok())
                        .unwrap_or(false)
                };

                if a_is_player && b_is_ia {
                    damage_queue.0.push(DamageEvent {
                        target: *ent_b,
                        amount: 10,
                    });
                }

                if b_is_player && a_is_ia {
                    damage_queue.0.push(DamageEvent {
                        target: *ent_a,
                        amount: 10,
                    });
                }
            }
        }
    }

    for res in to_resolve {
        apply_resolution(world, &res);
    }
}

const IA_SPEED: f64 = 200.0;
#[system(for_each)]
#[filter(component::<IA>())]
pub fn ia_seek(
    velo: &mut Velocity,
    pos: &Position,
    active: &Active,
    #[resource] pos_target: &PlayerPos,
    #[resource] dt: &Duration,
) {
    if !active.0 {
        return;
    }
    let dx = pos_target.x - pos.x;
    let dy = pos_target.y - pos.y;
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len > 0.0 {
        (dx / len, dy / len)
    } else {
        (0.0, 0.0)
    };

    velo.dx += nx * (*dt).as_secs_f64();
    velo.dy += ny * (*dt).as_secs_f64();
}

#[system]
#[write_component(Health)]
#[read_component(IA)]
#[read_component(Player)]
pub fn health(world: &mut SubWorld, #[resource] enemy_die_queue: &mut EnemyDiedQueue) {
    let dead: Vec<Entity> = <(Entity, &mut Health)>::query()
        .iter_mut(world)
        .filter(|(_, h)| h.hp == 0 && h.state != HealthState::Dead)
        .map(|(e, h)| {
            h.state = HealthState::Dead;
            *e
        })
        .collect();

    for entity in dead {
        if let Ok(entry) = world.entry_ref(entity) {
            if entry.get_component::<IA>().is_ok() {
                enemy_die_queue.0.push(EnemyDied(entity));
            }
            if entry.get_component::<Player>().is_ok() {
                // todo: Handle player death
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

const SPAWN_RADIUS: f64 = 800.0;
use std::f64::consts::PI;
#[system]
#[write_component(Health)]
#[write_component(Active)]
#[write_component(Position)]
pub fn wave_update(
    world: &mut SubWorld,
    #[resource] wave_manager: &mut WaveManager,
    #[resource] dt: &Duration,
    #[resource] wave_configs: &WaveConfigs,
    #[resource] player_pos: &PlayerPos,
    #[resource] enemy_die_queue: &mut EnemyDiedQueue,
    #[resource] enemy_pool: &EnemyPool,
) {
    match wave_manager.wave_state {
        WaveState::InProgress => {
            let remaining_spawn_time = wave_manager.spawn_timer.saturating_sub(*dt);
            wave_manager.spawn_timer = remaining_spawn_time;

            if remaining_spawn_time.is_zero() && wave_manager.enemies_to_spawn > 0 {
                for entity in enemy_pool.pool.iter() {
                    if let Ok(mut entry) = world.entry_mut(*entity) {
                        if let Ok(active) = entry.get_component_mut::<Active>() {
                            if active.0 {
                                continue; // Skip enemis actifs
                            } else {
                                *active = Active(true);
                                if let Ok(pos) = entry.get_component_mut::<Position>() {
                                    let angle = rand::random::<f64>() * 2.0 * PI;
                                    pos.x = player_pos.x + angle.cos() * SPAWN_RADIUS;
                                    pos.y = player_pos.y + angle.sin() * SPAWN_RADIUS;
                                }

                                if let Ok(health) = entry.get_component_mut::<Health>() {
                                    health.hp = wave_configs.0[wave_manager.current_wave].enemy_hp;
                                    health.state = HealthState::Alive;
                                }

                                wave_manager.spawn_timer = Duration::from_millis(
                                    wave_configs.0[wave_manager.current_wave].spawn_interval,
                                );
                                wave_manager.enemies_to_spawn -= 1;
                                break; // Spawn un ennemi à la fois
                            }
                        }
                    }
                } //end for
            } // Update spawn timer and enemy count

            for event in enemy_die_queue.0.iter() {
                wave_manager.enemies_remaining = wave_manager.enemies_remaining.saturating_sub(1);
                if let Ok(mut entry) = world.entry_mut(event.0) {
                    if let Ok(active) = entry.get_component_mut::<Active>() {
                        active.0 = false;
                    }
                }
            }

            if wave_manager.enemies_remaining == 0 && wave_manager.enemies_to_spawn == 0 {
                wave_manager.wave_state = WaveState::BetweenWave(Duration::from_secs(5));
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
                }
            } else {
                wave_manager.wave_state = WaveState::BetweenWave(remaining);
            }
        }
    }
}

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
                        if let Ok(value) = entry.get_component_mut::<CoinValue>() {
                            value.0 = 10;
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
    let mut query = <(Entity, &Position, &Collider, &Active)>::query();
    let entities: Vec<_> = query
        .iter(word)
        .filter(|(_, _, _, active)| active.0)
        .map(|(e, p, c, _)| (e, p, c))
        .collect();

    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ent_a, pos_a, col_a) = entities[i];
            let (ent_b, pos_b, col_b) = entities[j];

            if let Some(_) = aabb_overlap(pos_a, col_a, pos_b, col_b) {
                //println!("Collision detected between {:?} and {:?}", ent_a, ent_b);
                let a_is_player = {
                    word.entry_ref(*ent_a)
                        .map(|e| e.get_component::<Player>().is_ok())
                        .unwrap_or(false)
                };

                let b_is_player = {
                    word.entry_ref(*ent_b)
                        .map(|e| e.get_component::<Player>().is_ok())
                        .unwrap_or(false)
                };

                let a_is_coin = {
                    word.entry_ref(*ent_a)
                        .map(|e| e.get_component::<Coin>().is_ok())
                        .unwrap_or(false)
                };

                let b_is_coin = {
                    word.entry_ref(*ent_b)
                        .map(|e| e.get_component::<Coin>().is_ok())
                        .unwrap_or(false)
                };
                if a_is_player && b_is_coin {
                    pick_up_queue.0.push(*ent_b);
                }

                if b_is_player && a_is_coin {
                    pick_up_queue.0.push(*ent_a);
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
    #[resource] gold: &mut Gold,
) {
    for coin in pick_up_queue.0.iter() {
        if let Ok(mut entry) = world.entry_mut(*coin) {
            if let Ok(active) = entry.get_component_mut::<Active>() {
                *active = Active(false);
            }
            if let Ok(value) = entry.get_component::<CoinValue>() {
                gold.0 += value.0;
            }
        }
    }
}



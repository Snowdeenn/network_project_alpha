use std::collections::HashMap;
use std::time::Duration;

use crate::next_id;
use crate::simulation::components::*;
use crate::simulation::eco::{CoinPool, CoinSpawnQueue, PickupQueue, PlayerGold};
use crate::simulation::event::{
    CoinEvent, DamageEvent, DamageQueue, EnemyDied, EnemyDiedQueue, GameEventQueue,
};
use crate::simulation::helper::*;
use crate::simulation::wave::{EnemyPool, WaveConfigs, WaveManager, WaveState};
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::*;
use shared::protocol::{GameEvent, GameEventKind};

// todo: Ajouter plusieurs friction en fonction
// du milieu dans lequel le joueur ce déplace.

const FRICTION: f64 = 0.85;
const ARENA_W: f64 = 1920.0;
const ARENA_H: f64 = 1080.0;

pub fn spawn_player(
    world: &mut World,
    player_game_id: u64,
    class: PlayerClass,
    spawn_pos: Position,
) -> Entity {
    let (max_hp, collider, move_stats, attack_stats, attack_interval) = match class {
        PlayerClass::Warrior => (
            100,
            Collider { w: 40.0, h: 40.0 },
            MovementStats {
                accel: 1500.0,
                max_speed: 300.0,
            },
            AttackStats {
                range: 60.0,
                damage: 15,
                box_half_length: 25.0,
                box_half_width: 30.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.5),
        ),
        PlayerClass::Assassin => (
            75,
            Collider { w: 32.0, h: 32.0 },
            MovementStats {
                accel: 2200.0,
                max_speed: 400.0,
            },
            AttackStats {
                range: 50.0,
                damage: 25,
                box_half_length: 20.0,
                box_half_width: 20.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.3),
        ),
        PlayerClass::Mage => (
            80,
            Collider { w: 36.0, h: 36.0 },
            MovementStats {
                accel: 1200.0,
                max_speed: 250.0,
            },
            AttackStats {
                range: 300.0,
                damage: 18,
                box_half_length: 15.0,
                box_half_width: 15.0,
                projectile_speed: Some(400.0),
            },
            Duration::from_secs_f32(0.6),
        ),
        PlayerClass::Tank => (
            180,
            Collider { w: 48.0, h: 48.0 },
            MovementStats {
                accel: 900.0,
                max_speed: 200.0,
            },
            AttackStats {
                range: 55.0,
                damage: 10,
                box_half_length: 30.0,
                box_half_width: 45.0,
                projectile_speed: None,
            },
            Duration::from_secs_f32(0.7),
        ),
    };

    let entity = world.push((
        EntityId(player_game_id),
        Player,
        class,
        InputState::default(),
        spawn_pos,
        Velocity { dx: 0.0, dy: 0.0 },
        Dash(DashState::Idle),
        collider,
    ));

    let mut entry = world
        .entry(entity)
        .expect("[Spawner] Impossible de créer l'entry");
    entry.add_component(Active(true));
    entry.add_component(AttackTimer {
        remaining: Duration::ZERO,
        interval: attack_interval,
    });
    entry.add_component(move_stats);
    entry.add_component(attack_stats);
    entry.add_component(Health {
        hp: max_hp,
        max_hp,
        state: HealthState::Alive,
    });

    entity
}

// =================================================================================
// -------------------------------- PHYSIC SYSTEMS ---------------------------------
// =================================================================================

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
#[filter(!component::<Knockback>())]
pub fn update_velocity(
    velo: &mut Velocity,
    state: &InputState,
    mov_stats: &MovementStats,
    #[resource] dt: &Duration,
) {
    let input_x = state.move_dir[0] as f64 * mov_stats.accel * (*dt).as_secs_f64();
    let input_y = state.move_dir[1] as f64 * mov_stats.accel * (*dt).as_secs_f64();

    velo.dx += input_x;
    velo.dy += input_y;
}

#[system(for_each)]
#[filter(!component::<Projectile>())]
pub fn friction(velo: &mut Velocity) {
    velo.dx *= FRICTION;
    velo.dy *= FRICTION;
}

#[system(for_each)]
pub fn collide_arena(pos: &mut Position, col: &Collider) {
    pos.x = pos.x.clamp(0.0, ARENA_W - col.w);
    pos.y = pos.y.clamp(0.0, ARENA_H - col.h);
}

#[system(for_each)]
#[filter(component::<Projectile>())]
pub fn projectile_arena_culling(entity: &Entity, pos: &Position, command: &mut CommandBuffer) {
    const MARGIN: f64 = 100.0;
    if pos.x < -MARGIN || pos.x > ARENA_W + MARGIN || pos.y < -MARGIN || pos.y > ARENA_H + MARGIN {
        command.remove(*entity);
    }
}

#[system]
#[read_component(Collider)]
#[read_component(Player)]
#[read_component(IA)]
#[write_component(Velocity)]
#[write_component(Position)]
#[read_component(Active)]
pub fn collide(world: &mut SubWorld) {
    let mut query = <(Entity, &Position, &Collider, &Active)>::query().filter(!component::<Coin>());

    let entities: Vec<_> = query
        .iter(world)
        .filter(|(_, _, _, active)| active.0)
        .map(|(e, p, c, _)| (*e, *p, *c))
        .collect();

    let mut to_resolve: Vec<Resolution> = Vec::new();
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (ent_a, pos_a, col_a) = entities[i];
            let (ent_b, pos_b, col_b) = entities[j];

            if let Some((overlap_x, overlap_y)) = aabb_overlap(&pos_a, &col_a, &pos_b, &col_b) {
                let center_a_x = pos_a.x + col_a.w / 2.0;
                let center_b_x = pos_b.x + col_b.w / 2.0;
                let center_a_y = pos_a.y + col_a.h / 2.0;
                let center_b_y = pos_b.y + col_b.h / 2.0;

                to_resolve.push(Resolution {
                    ent_a,
                    ent_b,
                    overlap_x,
                    overlap_y,
                    dir_x: (center_a_x - center_b_x).signum(),
                    dir_y: (center_a_y - center_b_y).signum(),
                    axis: overlap_x < overlap_y,
                });
            }
        }
    }

    for res in to_resolve {
        apply_resolution(world, &res);
    }
}

// =================================================================================
// ---------------------------------- IA SYSTEMS -----------------------------------
// =================================================================================

#[system]
#[read_component(Player)]
#[read_component(Position)]
#[write_component(Target)]
#[read_component(Active)]
#[read_component(IA)]
pub fn ia_targeting(world: &mut SubWorld) {
    let mut player_query = <(Entity, &Position)>::query().filter(component::<Player>());
    let players: Vec<(Entity, Position)> =
        player_query.iter(world).map(|(e, p)| (*e, *p)).collect();

    let mut ia_query = <(&Position, &mut Target, &Active)>::query().filter(component::<IA>());

    for (ia_pos, target, active) in ia_query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        let mut closest_player = None;
        let mut min_dist = f64::MAX;

        for (p_entt, p_pos) in &players {
            let dx = p_pos.x - ia_pos.x;
            let dy = p_pos.y - ia_pos.y;
            let dist = dx * dx + dy * dy;

            if dist < min_dist {
                min_dist = dist;
                closest_player = Some(*p_entt);
            }
        }

        target.0 = closest_player;
    }
}

#[system]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Target)]
#[write_component(Velocity)]
pub fn ia_classic_movement(world: &mut SubWorld) {
    let player_positions: std::collections::HashMap<Entity, Position> = {
        let mut player_query = <(Entity, &Position)>::query()
            .filter(component::<Player>())
            .filter(!component::<Knockback>());
        player_query
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos))
            .collect()
    };

    let mut query =
        <(&Position, &Active, &Target, &mut Velocity)>::query().filter(component::<IA>());

    for (ia_pos, active, target, velo) in query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        if let Some(target_entity) = target.0 {
            if let Some(target_pos) = player_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > 45.0 {
                    velo.dx = (dx / distance) * 140.0;
                    velo.dy = (dy / distance) * 140.0;
                } else {
                    velo.dx = 0.0;
                    velo.dy = 0.0;
                }
            }
        } else {
            velo.dx = 0.0;
            velo.dy = 0.0;
        }
    }
}
// =================================================================================
// -------------------------------- HEALTH SYSTEMS ---------------------------------
// =================================================================================

#[system]
#[write_component(Health)]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(EntityId)]
pub fn health(
    world: &mut SubWorld,
    #[resource] enemy_die_queue: &mut EnemyDiedQueue,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    let dead: Vec<Entity> = <(Entity, &mut Health)>::query()
        .iter_mut(world)
        .filter(|(_, h)| h.hp == 0 && h.state != HealthState::Dead)
        .map(|(e, h)| {
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
                    .expect("[Heatlh System] Le joueur n'as pas le composant EntityId");
                game_event_queue.0.push(GameEvent {
                    kind: GameEventKind::PlayerDied { entity_id: id.0 },
                });
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

// =================================================================================
// -------------------------------- STATE SYSTEMS ----------------------------------
// =================================================================================

#[system(for_each)]
#[filter(component::<Player>())]
pub fn dash(
    velo: &mut Velocity,
    dash: &mut Dash,
    state: &InputState,
    #[resource] delta_time: &Duration,
) {
    let new_state = match dash.0 {
        DashState::Idle => {
            if state.dash {
                velo.dx *= 5.0;
                velo.dy *= 5.0;
                DashState::Dashing(Duration::from_millis(20))
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

// =================================================================================
// --------------------------------- WAVE SYSTEMS ----------------------------------
// =================================================================================

#[system]
#[write_component(Active)]
pub fn wave_death_reaper(
    world: &mut SubWorld,
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
pub fn wave_spawner(
    world: &mut SubWorld,
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

// =================================================================================
// --------------------------------- COIN SYSTEMS ----------------------------------
// =================================================================================

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

// =================================================================================
// -------------------------------- ATTACK SYSTEMS ---------------------------------
// =================================================================================

#[system]
#[read_component(Player)]
#[read_component(InputState)]
#[read_component(AttackStats)]
#[write_component(AttackTimer)]
pub fn read_player_attack_intent(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] dt: &Duration,
) {
    let mut query = <(Entity, &InputState, &AttackStats, &mut AttackTimer)>::query()
        .filter(component::<Player>());

    for (entity, state, stats, timer) in query.iter_mut(world) {
        timer.remaining = timer.remaining.saturating_sub(*dt);

        if state.attack && timer.remaining.is_zero() {
            command.add_component(
                *entity,
                AttackIntent {
                    aim_dir: state.aim_dir,
                    box_half_length: stats.box_half_length,
                    box_half_width: stats.box_half_width,
                    projectile_speed: stats.projectile_speed,
                    damage: stats.damage,
                },
            );
            timer.remaining = timer.interval;
        }
    }
}

#[system]
#[read_component(IA)]
#[read_component(Player)]
#[read_component(Active)]
#[read_component(Position)]
#[read_component(Target)]
#[read_component(AttackStats)]
#[write_component(AttackTimer)]
pub fn ia_classic_attack(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] dt: &Duration,
) {
    let player_positions: std::collections::HashMap<Entity, Position> = {
        let mut player_query = <(Entity, &Position)>::query().filter(component::<Player>());
        player_query
            .iter(&*world)
            .map(|(entity, pos)| (*entity, *pos))
            .collect()
    };

    let mut query = <(
        Entity,
        &Position,
        &Active,
        &AttackStats,
        &Target,
        &mut AttackTimer,
    )>::query()
    .filter(component::<IA>());

    for (entity, ia_pos, active, stats, target, timer) in query.iter_mut(world) {
        if !active.0 {
            continue;
        }

        timer.remaining = timer.remaining.saturating_sub(*dt);

        if let Some(target_entity) = target.0 {
            if let Some(target_pos) = player_positions.get(&target_entity) {
                let dx = target_pos.x - ia_pos.x;
                let dy = target_pos.y - ia_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < 60.0 && timer.remaining.is_zero() {
                    command.add_component(
                        *entity,
                        AttackIntent {
                            aim_dir: [(dx / distance) as f32, (dy / distance) as f32],
                            box_half_length: stats.box_half_length,
                            box_half_width: stats.box_half_width,
                            projectile_speed: stats.projectile_speed,
                            damage: stats.damage,
                        },
                    );
                    timer.remaining = timer.interval;
                }
            }
        }
    }
}

const OFFSET_ATTACKBOX: f32 = 10.0;
const PLAYER_RADIUS: f32 = 20.0;

#[system(for_each)]
#[filter(component::<AttackIntent>())]
pub fn create_attack_box(
    entity: &Entity,
    pos: &Position,
    intent: &AttackIntent,
    command: &mut CommandBuffer,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    let dir = intent.aim_dir;

    if let Some(speed) = intent.projectile_speed {
        let entity = command.push((
            EntityId(next_id()),
            Position { x: pos.x, y: pos.y },
            Velocity {
                dx: dir[0] as f64 * speed,
                dy: dir[1] as f64 * speed,
            },
            Geometry {
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
            Damage(intent.damage),
            TeamFilter { is_player: true }, // Pour que tes IA prennent les dégâts
            Owner(*entity),
            Projectile,
        ));
        command.add_component(entity, Active(true));
    } else {
        let dist_to_center =
            (PLAYER_RADIUS + OFFSET_ATTACKBOX + intent.box_half_length as f32) as f64;
        let center_x = pos.x + (dir[0] as f64 * dist_to_center);
        let center_y = pos.y + (dir[1] as f64 * dist_to_center);

        command.push((
            Position {
                x: center_x,
                y: center_y,
            },
            Geometry {
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
            Damage(intent.damage),
            TeamFilter { is_player: true },
            Owner(*entity), // On garde l'owner pour ton système actuel
            Active(true),
        ));

        // Rendu Debug via ton événement réseau existant !
        game_event_queue.0.push(GameEvent {
            kind: GameEventKind::DebugRect {
                x: center_x as f32,
                y: center_y as f32,
                half_length: intent.box_half_length as f32,
                half_width: intent.box_half_width as f32,
                dir,
            },
        });
    }

    command.remove_component::<AttackIntent>(*entity);
}

#[system]
#[read_component(Player)]
#[read_component(IA)]
#[read_component(Projectile)]
#[read_component(Collider)]
#[read_component(Geometry)]
#[read_component(Position)]
#[read_component(Owner)]
#[read_component(Health)]
#[read_component(Damage)]
pub fn check_collide_attackbox(
    world: &mut SubWorld,
    command: &mut CommandBuffer,
    #[resource] damage_queue: &mut DamageQueue,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    let players: std::collections::HashSet<Entity> = <Entity>::query()
        .filter(component::<Player>())
        .iter(world)
        .copied()
        .collect();

    let mut attackbox_query = <(Entity, &Geometry, &Owner, &Damage, &Position)>::query();
    let attackboxes: Vec<_> = attackbox_query
        .iter(world)
        .map(|(e, g, o, d, p)| (*e, *g, *o, *d, *p))
        .collect();

    let mut victim_query = <(Entity, &Collider, &Position)>::query().filter(component::<Health>());
    let victims: Vec<_> = victim_query
        .iter(world)
        .map(|(e, c, p)| (*e, *c, *p))
        .collect();

    for (attackbox_entt, attackbox, owner, damage, attackbox_pos) in attackboxes {
        let attacker_is_player = players.contains(&owner.0);
        let is_projectile = world
            .entry_ref(attackbox_entt)
            .map(|e| e.get_component::<Projectile>().is_ok())
            .unwrap_or(false);
        let mut hit = false;

        for (victim_entt, victim_col, victim_pos) in &victims {
            if *victim_entt == owner.0 {
                continue;
            }
            if obb_vs_aabb(&attackbox_pos, &attackbox, victim_pos, victim_col) {
                let mut should_damage = false;

                if attacker_is_player {
                    should_damage = true;
                } else {
                    let victim_is_player = players.contains(victim_entt);
                    if victim_is_player {
                        should_damage = true;
                    }
                }

                if should_damage {
                    damage_queue.0.push(DamageEvent {
                        target: *victim_entt,
                        amount: damage.0,
                    });
                    game_event_queue.0.push(GameEvent {
                        kind: GameEventKind::EntityHit {
                            pos: [victim_pos.x as f32, victim_pos.y as f32],
                        },
                    });

                    {
                        let mut dx = victim_pos.x - attackbox_pos.x;
                        let mut dy = victim_pos.y - attackbox_pos.y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > 0.0 {
                            dx /= distance;
                            dy /= distance;
                        } else {
                            dx = 1.0;
                            dy = 0.0;
                        }

                        let knockback_force = 600.0f32;
                        let knockback_duration = 0.12;

                        command.add_component(
                            *victim_entt,
                            Knockback {
                                dx: dx as f32 * knockback_force,
                                dy: dy as f32 * knockback_force,
                                duration: knockback_duration,
                            },
                        );
                        hit = true;
                    }
                }
            }
        }
        if !is_projectile || hit {
            command.remove(attackbox_entt);
        }
    }
}

// =================================================================================
// --------------------------------                ---------------------------------
// =================================================================================

#[system(for_each)]
#[filter(component::<Knockback>())]
pub fn knockback(
    entt: &Entity,
    kb: &mut Knockback,
    velo: &mut Velocity,
    #[resource] dt: &Duration,
    command: &mut CommandBuffer,
) {
    velo.dx += kb.dx as f64;
    velo.dy += kb.dy as f64;

    const MAX_VELO: f64 = 600.0;
    velo.dx = velo.dx.clamp(-MAX_VELO, MAX_VELO);
    velo.dy = velo.dy.clamp(-MAX_VELO, MAX_VELO);

    kb.dx = 0.0;
    kb.dy = 0.0;
    kb.duration -= dt.as_secs_f32();

    if kb.duration <= 0.0 {
        command.remove_component::<Knockback>(*entt);
    }
}

// =================================================================================
// -------------------------------- Debug Systems ----------------------------------
// =================================================================================

#[system(for_each)]
#[filter(component::<Player>())]
pub fn send_collider(pos: &Position, #[resource] game_event_queue: &mut GameEventQueue) {
    game_event_queue.0.push(GameEvent {
        kind: GameEventKind::DebugCollider {
            x: pos.x as f32,
            y: pos.y as f32,
        },
    });
}

#[system(for_each)]
#[filter(component::<Projectile>())] // S'applique uniquement aux projectiles
pub fn debug_projectile_positions(
    pos: &Position,
    geo: &Geometry,
    #[resource] game_event_queue: &mut GameEventQueue,
) {
    // On envoie un point de débug à la position du projectile
    game_event_queue.0.push(GameEvent {
        kind: GameEventKind::DebugRect {
            x: pos.x as f32,
            y: pos.y as f32,
            half_length: geo.half_length,
            half_width: geo.half_width,
            dir: geo.dir,
        },
    });
}

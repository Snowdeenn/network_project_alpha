use legion::world::Entity;
use shared::protocol::GameEvent;
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: u32,
}

#[derive(Debug)]
pub struct DamageQueue(pub Vec<DamageEvent>);

#[derive(Debug)]
pub struct EnemyDied(pub Entity);
#[derive(Debug)]
pub struct EnemyDiedQueue(pub Vec<EnemyDied>);

#[derive(Debug)]
pub struct PlayerDied(pub bool);

#[derive(Debug)]
pub struct CoinEvent {
    pub pos: [f32; 2],
}

#[derive(Debug)]
pub struct GameEventQueue(pub Vec<GameEvent>);

use legion::world::Entity;
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: u32,
}

#[derive(Debug)]
pub struct EnemyDied(pub Entity);

#[derive(Debug)]
pub struct CoinEvent {
    pub pos: [f32; 2],
}


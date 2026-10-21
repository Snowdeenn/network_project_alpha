use super::instance::VfxInstanceId;
use math::Vec2;
use std::collections::HashMap;
const PARTICLE_POOL_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub friction: f32,
    pub lifetime: f32,
    pub lt_max: f32,
    pub disappearance_seconds: f32,
    pub scale: f32,
    pub growth: f32,
    pub color: crate::rendering::Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceleration_moves_particles_and_drag_does_not_reverse_them() {
        let mut pool = ParticlePool::new();
        let mut particle = pool.slots[0].data;
        particle.lifetime = 10.0;
        particle.acceleration = Vec2::new(2.0, 0.0);
        pool.spawn(VfxInstanceId::zero(), particle);
        pool.update(0.5);
        assert!((pool.slots[0].data.velocity.x - 1.0).abs() < 0.00001);
        assert!((pool.slots[0].data.pos.x - 0.5).abs() < 0.00001);

        let p = &mut pool.slots[0].data;
        p.acceleration = Vec2::zero();
        p.friction = 10.0;
        pool.update(0.5);
        assert!(pool.slots[0].data.velocity.x > 0.0);
        assert!(pool.slots[0].data.velocity.x < 1.0);
    }
}

struct Slot {
    active: bool,
    owner: Option<VfxInstanceId>,
    data: Particle,
}

pub struct ParticlePool {
    slots: Vec<Slot>,
    live_counts: HashMap<VfxInstanceId, usize>,
}

impl ParticlePool {
    pub(super) fn iter(&self) -> impl Iterator<Item = (VfxInstanceId, &Particle)> {
        self.slots.iter().filter_map(|slot| {
            slot.owner
                .filter(|_| slot.active)
                .map(|owner| (owner, &slot.data))
        })
    }

    pub fn new() -> Self {
        let dummy = Particle {
            pos: Vec2::zero(),
            velocity: Vec2::zero(),
            acceleration: Vec2::zero(),
            friction: 0.0,
            lifetime: 0.0,
            lt_max: 1.0,
            disappearance_seconds: 0.0,
            scale: 0.0,
            growth: 0.0,
            color: crate::rendering::Color::WHITE,
        };

        Self {
            slots: (0..PARTICLE_POOL_SIZE)
                .map(|_| Slot {
                    active: false,
                    owner: None,
                    data: dummy,
                })
                .collect(),
            live_counts: HashMap::new(),
        }
    }

    /// Acquiert le premier slot libre et y écrit la particule.
    /// Si le pool est plein, la particule est silencieusement ignorée.
    pub fn spawn(&mut self, owner: VfxInstanceId, p: Particle) {
        if let Some(slot) = self.slots.iter_mut().find(|s| !s.active) {
            slot.active = true;
            slot.data = p;
            slot.owner = Some(owner);
            *self.live_counts.entry(owner).or_default() += 1;
        }
    }

    pub fn live_count(&self, owner: VfxInstanceId) -> usize {
        self.live_counts.get(&owner).copied().unwrap_or(0)
    }

    pub fn remove_owner(&mut self, owner: VfxInstanceId) {
        for slot in &mut self.slots {
            if slot.owner == Some(owner) {
                slot.active = false;
                slot.owner = None;
            }
        }
        self.live_counts.remove(&owner);
    }

    pub fn update(&mut self, dt: f32) {
        assert!(dt.is_finite() && dt >= 0.0);
        for slot in &mut self.slots {
            if !slot.active {
                continue;
            }

            let p = &mut slot.data;
            p.lifetime -= dt;

            if p.lifetime <= -p.disappearance_seconds {
                slot.active = false;
                let owner = slot.owner.take().unwrap();
                let count = self.live_counts.get_mut(&owner).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.live_counts.remove(&owner);
                }
                continue;
            }
            // La mort logique arrête la simulation, mais le slot reste réservé
            // jusqu'à la fin de la disparition visuelle.
            if p.lifetime <= 0.0 {
                continue;
            }

            p.velocity += p.acceleration * dt;
            p.velocity = p.velocity * (-p.friction * dt).exp();
            p.pos += p.velocity * dt;
            p.scale += p.growth * dt;
        }
    }
}

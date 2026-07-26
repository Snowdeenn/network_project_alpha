use raylib::prelude::*;

const PARTICLE_POOL_SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vector2,
    pub velocity: Vector2,
    pub friction: f32,
    pub lifetime: f32,
    pub lt_max: f32,
    pub scale: f32,
    pub growth: f32,
    pub color: Color,
}

struct Slot {
    active: bool,
    data: Particle,
}

pub struct ParticlePool {
    slots: Vec<Slot>,
}

impl ParticlePool {
    pub fn new() -> Self {
        let dummy = Particle {
            pos: Vector2::zero(),
            velocity: Vector2::zero(),
            friction: 0.0,
            lifetime: 0.0,
            lt_max: 1.0,
            scale: 0.0,
            growth: 0.0,
            color: Color::WHITE,
        };

        Self {
            slots: (0..PARTICLE_POOL_SIZE)
                .map(|_| Slot { active: false, data: dummy })
                .collect(),
        }
    }

    /// Acquiert le premier slot libre et y écrit la particule.
    /// Si le pool est plein, la particule est silencieusement ignorée.
    pub fn spawn(&mut self, p: Particle) {
        if let Some(slot) = self.slots.iter_mut().find(|s| !s.active) {
            slot.active = true;
            slot.data = p;
        }
    }

    pub fn update(&mut self, dt: f32) {
        for slot in &mut self.slots {
            if !slot.active {
                continue;
            }

            let p = &mut slot.data;
            p.lifetime -= dt;

            if p.lifetime <= 0.0 {
                slot.active = false;
                continue;
            }

            p.pos += p.velocity * dt;
            p.velocity.x *= 1.0 - (p.friction * dt);
            p.velocity.y *= 1.0 - (p.friction * dt);
            p.scale += p.growth * dt;
        }
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut RaylibMode2D<D>) {
        for slot in &self.slots {
            if !slot.active {
                continue;
            }

            let p = &slot.data;
            let size = (16.0 * p.scale) as i32;
            let progress = (p.lifetime / p.lt_max).clamp(0.0, 1.0);

            d.draw_rectangle(
                p.pos.x as i32 - (size / 2),
                p.pos.y as i32 - (size / 2),
                size,
                size,
                p.color.alpha(progress),
            );
        }
    }
}

#[cfg(test)]
mod tests_particle {
    use super::*;

    fn dummy_particle(lifetime: f32) -> Particle {
        Particle {
            pos: Vector2::zero(),
            velocity: Vector2::new(10.0, 0.0),
            friction: 0.0,
            lifetime,
            lt_max: lifetime,
            scale: 0.1,
            growth: 1.0,
            color: Color::WHITE,
        }
    }

    #[test]
    fn spawn_active_after_insert() {
        let mut pool = ParticlePool::new();
        pool.spawn(dummy_particle(1.0));
        let active = pool.slots.iter().filter(|s| s.active).count();
        assert_eq!(active, 1);
    }

    #[test]
    fn pool_full_drops_extra() {
        let mut pool = ParticlePool::new();
        for _ in 0..(PARTICLE_POOL_SIZE + 10) {
            pool.spawn(dummy_particle(1.0));
        }
        let active = pool.slots.iter().filter(|s| s.active).count();
        assert_eq!(active, PARTICLE_POOL_SIZE);
    }

    #[test]
    fn update_decrements_lifetime() {
        let mut pool = ParticlePool::new();
        pool.spawn(dummy_particle(1.0));
        pool.update(0.3);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!((p.data.lifetime - 0.7).abs() < 1e-4);
    }

    #[test]
    fn update_releases_expired() {
        let mut pool = ParticlePool::new();
        pool.spawn(dummy_particle(0.1));
        pool.update(0.2);
        let active = pool.slots.iter().filter(|s| s.active).count();
        assert_eq!(active, 0);
    }

    #[test]
    fn update_moves_particle() {
        let mut pool = ParticlePool::new();
        pool.spawn(Particle {
            pos: Vector2::zero(),
            velocity: Vector2::new(100.0, 0.0),
            friction: 0.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 0.0,
            color: Color::WHITE,
        });
        pool.update(0.1);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!((p.data.pos.x - 10.0).abs() < 1e-3);
    }

    #[test]
    fn slot_reused_after_expiry() {
        let mut pool = ParticlePool::new();
        pool.spawn(dummy_particle(0.05));
        pool.update(0.1); // expire

        pool.spawn(dummy_particle(1.0)); // doit réutiliser le slot libéré
        let active = pool.slots.iter().filter(|s| s.active).count();
        assert_eq!(active, 1);
    }

    #[test]
    fn friction_slows_velocity() {
        let mut pool = ParticlePool::new();
        pool.spawn(Particle {
            pos: Vector2::zero(),
            velocity: Vector2::new(100.0, 0.0),
            friction: 5.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 0.0,
            color: Color::WHITE,
        });
        pool.update(0.1);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!(p.data.velocity.x < 100.0);
    }

    #[test]
    fn growth_increases_scale() {
        let mut pool = ParticlePool::new();
        pool.spawn(Particle {
            pos: Vector2::zero(),
            velocity: Vector2::zero(),
            friction: 0.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 10.0,
            color: Color::WHITE,
        });
        pool.update(0.1);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!(p.data.scale > 0.1);
    }
}
use utils::math::Vec2;
const PARTICLE_POOL_SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub friction: f32,
    pub lifetime: f32,
    pub lt_max: f32,
    pub scale: f32,
    pub growth: f32,
    pub color: utils::colors::Color,
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
            pos: Vec2::zero(),
            velocity: Vec2::zero(),
            friction: 0.0,
            lifetime: 0.0,
            lt_max: 1.0,
            scale: 0.0,
            growth: 0.0,
            color: utils::colors::Color::WHITE,
        };

        Self {
            slots: (0..PARTICLE_POOL_SIZE)
                .map(|_| Slot {
                    active: false,
                    data: dummy,
                })
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

    pub fn push_draw_commands(&self, frame: &mut prism::Frame) {
        for slot in &self.slots {
            if !slot.active {
                continue;
            }

            let p = &slot.data;
            let size = 16.0 * p.scale;
            let progress = (p.lifetime / p.lt_max).clamp(0.0, 1.0);
            let base_alpha = (p.color.a as f32) / 255.0;
            let alpha = base_alpha * progress;

            frame.push_vfx(prism::DrawCommand::Shape {
                shape: prism::Shape::Quad {
                    pos: [p.pos.x - (size / 2.0), p.pos.y - (size / 2.0)],
                    size: [size; 2],
                    rotation: 0.0,
                    color: [
                        (p.color.r as f32) / 255.0,
                        (p.color.g as f32) / 255.0,
                        (p.color.b as f32) / 255.0,
                        alpha,
                    ],
                    uv: None,
                },
                blend: prism::BlendMode::Alpha,
                layer: 1,
            });
        }
    }
}

#[cfg(test)]
mod tests_particle {
    use super::*;

    fn dummy_particle(lifetime: f32) -> Particle {
        Particle {
            pos: Vec2::zero(),
            velocity: Vec2::new(10.0, 0.0),
            friction: 0.0,
            lifetime,
            lt_max: lifetime,
            scale: 0.1,
            growth: 1.0,
            color: utils::colors::Color::WHITE,
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
            pos: Vec2::zero(),
            velocity: Vec2::new(100.0, 0.0),
            friction: 0.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 0.0,
            color: utils::colors::Color::WHITE,
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
            pos: Vec2::zero(),
            velocity: Vec2::new(100.0, 0.0),
            friction: 5.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 0.0,
            color: utils::colors::Color::WHITE,
        });
        pool.update(0.1);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!(p.data.velocity.x < 100.0);
    }

    #[test]
    fn growth_increases_scale() {
        let mut pool = ParticlePool::new();
        pool.spawn(Particle {
            pos: Vec2::zero(),
            velocity: Vec2::zero(),
            friction: 0.0,
            lifetime: 1.0,
            lt_max: 1.0,
            scale: 0.1,
            growth: 10.0,
            color: utils::colors::Color::WHITE,
        });
        pool.update(0.1);
        let p = pool.slots.iter().find(|s| s.active).unwrap();
        assert!(p.data.scale > 0.1);
    }
}

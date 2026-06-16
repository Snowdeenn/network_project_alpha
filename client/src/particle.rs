use raylib::prelude::*;

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ParticleSystem {
    pub particle_pool: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            particle_pool: Vec::new(),
        }
    }

    pub fn spawn(&mut self, particle: Particle) {
        self.particle_pool.push(particle);
    }

    pub fn update(&mut self, dt: f32) {
        self.particle_pool.retain_mut(|p| {
            p.lifetime -= dt;
            p.pos += p.velocity * dt;
            
            p.velocity.x *= 1.0 - (p.friction * dt);
            p.velocity.y *= 1.0 - (p.friction * dt);

            p.scale += p.growth * dt;

            p.lifetime > 0.0
        });
    }

    pub fn draw(&self, d: &mut RaylibMode2D<RaylibDrawHandle>) {
        for p in &self.particle_pool {
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
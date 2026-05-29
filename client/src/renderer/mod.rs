pub mod hud;

use raylib::prelude::*;
use shared::protocol::{EntityKind, StateSnapshot};

pub struct Renderer {
    pub rl:     RaylibHandle,
    pub thread: RaylibThread,
    pub cam:    Camera2D,
    screen_w:   i32,
    screen_h:   i32,
}

impl Renderer {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let (mut rl, thread) = raylib::init()
            .size(screen_w, screen_h)
            .title("Project Alpha")
            .build();
        rl.set_target_fps(60);

        let cam = Camera2D {
            offset:   Vector2::new(screen_w as f32 / 2.0, screen_h as f32 / 2.0),
            target:   Vector2::zero(),
            rotation: 0.0,
            zoom:     1.0,
        };

        Self { rl, thread, cam, screen_w, screen_h }
    }

    pub fn render_frame(&mut self, snapshot: Option<&StateSnapshot>) {
        let mut d = self.rl.begin_drawing(&self.thread);
        d.clear_background(Color::BLACK);

        {
            let mut d2 = d.begin_mode2D(self.cam);
            match snapshot {
                None       => { d2.draw_text("Connexion...", -80, -10, 20, Color::WHITE); }
                Some(snap) => render_world(&mut d2, snap),
            }
        }

        if let Some(snap) = snapshot {
            hud::render(&mut d, snap);
        }
        d.draw_fps(self.screen_w - 100, 20);
    }
}

fn render_world(d: &mut RaylibMode2D<RaylibDrawHandle>, snap: &StateSnapshot) {
    for entity in &snap.entities {
        match &entity.entity_kind {
            EntityKind::Player => {
                d.draw_rectangle(
                    entity.position[0] as i32 - 20,
                    entity.position[1] as i32 - 20,
                    40, 40, Color::SKYBLUE,
                );
            }
            EntityKind::Enemy => {
                d.draw_rectangle(
                    entity.position[0] as i32 - 20,
                    entity.position[1] as i32 - 20,
                    40, 40, Color::RED,
                );
                let bar_w = 40.0 * (entity.health / entity.max_health);
                d.draw_rectangle(entity.position[0] as i32 - 20, entity.position[1] as i32 - 30, 40, 5, Color::DARKGRAY);
                d.draw_rectangle(entity.position[0] as i32 - 20, entity.position[1] as i32 - 30, bar_w as i32, 5, Color::GREEN);
            }
            EntityKind::Boss(_) => {
                d.draw_rectangle(
                    entity.position[0] as i32 - 40,
                    entity.position[1] as i32 - 40,
                    80, 80, Color::PURPLE,
                );
            }
            EntityKind::Projectile => {
                d.draw_circle(entity.position[0] as i32, entity.position[1] as i32, 8.0, Color::YELLOW);
            },
            EntityKind::Coin => println!("coin"),
        }
    }
}
pub mod hud;

use crate::TICK_DURATION;
use crate::config::*;
use crate::event::ClientState;
use raylib::prelude::*;
use shared::protocol::{EntityKind, StateSnapshot};
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct ScreenScale {
    pub w: f32,
    pub h: f32,
}

impl ScreenScale {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w: w as f32,
            h: h as f32,
        }
    }

    pub fn x(&self, ratio: f32) -> i32 {
        (ratio * self.w) as i32
    }
    pub fn y(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }
    pub fn w(&self, ratio: f32) -> i32 {
        (ratio * self.w) as i32
    }
    pub fn h(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }
    pub fn font(&self, ratio: f32) -> i32 {
        (ratio * self.h) as i32
    }
}

pub struct Renderer {
    pub rl: RaylibHandle,
    pub thread: RaylibThread,
    pub cam: Camera2D,
    screen_w: i32,
    screen_h: i32,
    screen_scale: ScreenScale,
}

impl Renderer {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let (mut rl, thread) = raylib::init()
            .size(screen_w, screen_h)
            .title("Project Alpha")
            .fullscreen()
            .build();
        rl.set_target_fps(120);

        let real_w = rl.get_screen_width();
        let real_h = rl.get_screen_height();
        
        let zoom = real_h as f32 / REFERENCE_H;

        let cam = Camera2D {
            offset: Vector2::new(real_w as f32 / 2.0, real_h as f32 / 2.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom,
        };

        Self {
            rl,
            thread,
            cam,
            screen_w: real_w,
            screen_h: real_h,
            screen_scale: ScreenScale::new(real_w, real_h),
        }
    }

    pub fn render_frame(
        &mut self,
        prev: Option<&StateSnapshot>,
        current: Option<&StateSnapshot>,
        last_snap_time: Instant,
        client_state: &ClientState,
    ) {
        let t =
            (last_snap_time.elapsed().as_secs_f32() / TICK_DURATION.as_secs_f32()).clamp(0.0, 1.0);
        let s = &self.screen_scale;

        let mut d = self.rl.begin_drawing(&self.thread);
        d.clear_background(Color::BLACK);

        {
            let mut d2 = d.begin_mode2D(self.cam);
            match current {
                None => {
                    d2.draw_text("Connexion...", -80, -10, 20, Color::WHITE);
                }
                Some(curr) => render_world(&mut d2, prev, curr, t),
            }
        }

        if let Some(snap) = current {
            hud::render(&mut d, snap, s);
        }

        if client_state.shop_available && !client_state.show_shop {
            d.draw_text(
                "Shop disponible — appuie sur G",
                s.x(HUD_SHOP_NOTIF_X),
                s.y(HUD_SHOP_NOTIF_Y),
                s.font(HUD_SHOP_NOTIF_FONT),
                Color::GOLD,
            );
        }

        hud::render_shop(&mut d, client_state, s);

        d.draw_fps(self.screen_w - 100, 20);
    }
}

fn render_world(
    d: &mut RaylibMode2D<RaylibDrawHandle>,
    prev: Option<&StateSnapshot>,
    curr: &StateSnapshot,
    t: f32,
) {
    for entity in &curr.entities {
        let prev_entity =
            prev.and_then(|p| p.entities.iter().find(|e| e.entity_id == entity.entity_id));

        let (x, y) = match prev_entity {
            Some(prev) => (
                lerp(prev.position[0], entity.position[0], t),
                lerp(prev.position[1], entity.position[1], t),
            ),
            None => (entity.position[0], entity.position[1]), // spawn → pas d'interpolation
        };
        match &entity.entity_kind {
            EntityKind::Player => {
                d.draw_rectangle(x as i32 - 20, y as i32 - 20, 40, 40, Color::SKYBLUE);
            }
            EntityKind::Enemy => {
                d.draw_rectangle(x as i32 - 20, y as i32 - 20, 40, 40, Color::RED);
                let bar_w = 40.0 * (entity.health / entity.max_health);
                d.draw_rectangle(x as i32 - 20, y as i32 - 30, 40, 5, Color::DARKGRAY);
                d.draw_rectangle(x as i32 - 20, y as i32 - 30, bar_w as i32, 5, Color::GREEN);
            }
            EntityKind::Boss(_) => {
                d.draw_rectangle(x as i32 - 40, y as i32 - 40, 80, 80, Color::PURPLE);
            }
            EntityKind::Projectile => {
                d.draw_circle(x as i32, y as i32, 8.0, Color::YELLOW);
            }
            EntityKind::Coin => {
                d.draw_circle(x as i32, y as i32, 10.0, Color::GOLD);
            }
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

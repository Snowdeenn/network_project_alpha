// Dans rendering/backend/raylib.rs
use utils::{colors, math};

pub fn to_raylib_color(c: colors::Color) -> raylib::prelude::Color {
    raylib::prelude::Color::new(c.r, c.g, c.b, c.a)
}

pub fn to_raylib_vec2(v: math::vec2::Vec2) -> raylib::math::Vector2 {
    raylib::math::Vector2::new(v.x, v.y)
}
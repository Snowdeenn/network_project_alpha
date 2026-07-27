use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x: x, y: y }
    }
    pub fn zero() -> Vec2 {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn dot(&self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    pub fn normalize(&self) -> Vec2 {
        let x: f32 = self.x / self.length();
        let y: f32 = self.y / self.length();

        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        let x: f32 = self.x + other.x;
        let y: f32 = self.y + other.y;

        Vec2 { x, y }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        let x: f32 = self.x - other.x;
        let y: f32 = self.y - other.y;

        Vec2 { x, y }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, other: f32) -> Vec2 {
        let x: f32 = self.x * other;
        let y: f32 = self.y * other;

        Vec2 { x, y }
    }
}

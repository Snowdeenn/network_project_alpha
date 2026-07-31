use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
    pub fn zero() -> Vec3 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn dot(&self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    pub fn normalize(&self) -> Vec3 {
        let x: f32 = self.x / self.length();
        let y: f32 = self.y / self.length();
        let z: f32 = self.z / self.length();

        Vec3 { x, y, z }
    }

    pub fn cross(&self, other: Vec3) -> Vec3 {
        let x: f32 = self.y * other.z - self.z * other.y;
        let y: f32 = self.z * other.x - self.x * other.z;
        let z: f32 = self.x * other.y - self.y * other.x;

        Vec3 { x, y, z }
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Vec3 {
        let x: f32 = self.x + other.x;
        let y: f32 = self.y + other.y;
        let z: f32 = self.z + other.z;

        Vec3 { x, y, z }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Vec3 {
        let x: f32 = self.x - other.x;
        let y: f32 = self.y - other.y;
        let z: f32 = self.z - other.z;

        Vec3 { x, y, z }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, other: f32) -> Vec3 {
        let x: f32 = self.x * other;
        let y: f32 = self.y * other;
        let z: f32 = self.z * other;

        Vec3 { x, y, z }
    }
}

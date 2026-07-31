use crate::math::vec3::Vec3;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn zero() -> Vec4 {
        Vec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        }
    }

    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
        Vec4 { x, y, z, w }
    }

    pub fn direction(vec: Vec3) -> Vec4 {
        Vec4 {
            x: vec.x,
            y: vec.y,
            z: vec.z,
            w: 0.0,
        }
    }

    pub fn point(vec: Vec3) -> Vec4 {
        Vec4 {
            x: vec.x,
            y: vec.y,
            z: vec.z,
            w: 1.0,
        }
    }

    pub fn dot(&self, other: Vec4) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    pub fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    pub fn normalize(&self) -> Vec4 {
        let length: f32 = self.length();

        let x: f32 = self.x / length;
        let y: f32 = self.y / length;
        let z: f32 = self.z / length;
        let w: f32 = self.w / length;

        Vec4 { x, y, z, w }
    }
}

impl Add for Vec4 {
    type Output = Vec4;

    fn add(self, other: Vec4) -> Vec4 {
        let x: f32 = self.x + other.x;
        let y: f32 = self.y + other.y;
        let z: f32 = self.z + other.z;
        let w: f32 = self.w + other.w;

        Vec4 { x, y, z, w }
    }
}
impl Sub for Vec4 {
    type Output = Vec4;

    fn sub(self, other: Vec4) -> Vec4 {
        let x: f32 = self.x - other.x;
        let y: f32 = self.y - other.y;
        let z: f32 = self.z - other.z;
        let w: f32 = self.w - other.w;

        Vec4 { x, y, z, w }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Vec4;

    fn mul(self, other: f32) -> Vec4 {
        let x: f32 = self.x * other;
        let y: f32 = self.y * other;
        let z: f32 = self.z * other;
        let w: f32 = self.w * other;

        Vec4 { x, y, z, w }
    }
}

//! Uniform types for the game's effects, and the blind visibility calculation.
//! Other effects define their own uniforms where they register their shaders.
#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Clone, Copy)]
pub struct HitFlashUniform {
    pub intensity: f32,
}

#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Clone, Copy)]
pub struct BlindUniform {
    pub amount: f32,
    pub aspect_ratio: f32,
}

pub fn blind_vision_radius(intensity: f32, width: f32, height: f32) -> f32 {
    let full_radius = (width * width + height * height).sqrt() * 0.5;
    full_radius + (0.1 * height - full_radius) * intensity.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vision_radius_tracks_shader_and_reveals_the_full_view_at_expiration() {
        assert_eq!(blind_vision_radius(1.0, 800.0, 600.0), 60.0);
        assert_eq!(blind_vision_radius(0.5, 800.0, 600.0), 280.0);
        assert_eq!(blind_vision_radius(0.0, 800.0, 600.0), 500.0);
    }
}

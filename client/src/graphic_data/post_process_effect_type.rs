#[derive(Clone, Copy)]
pub struct HitFlashEffect {
    pub id: prism::PostProcessPassId,
    pub timer: f32,
    pub total_duration: f32,
    pub intensity: f32,
}

pub fn update_hit_flash(hit_flash: &mut HitFlashEffect, dt: f32) {
    hit_flash.timer = (hit_flash.timer - dt).max(0.0);
    hit_flash.intensity = hit_flash.timer / hit_flash.total_duration;
}

#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Clone, Copy)]
pub struct HitFlashUniform {
    pub intensity: f32,
}

#[derive(Clone, Copy)]
pub struct BlindEffect {
    pub id: prism::PostProcessPassId,
    pub timer: f32,
    pub total_duration: f32,
    pub amount: f32,
    pub aspect_ratio: f32,
}

impl BlindEffect {
    pub fn intensity(&self) -> f32 {
        let elapsed = self.total_duration - self.timer;
        let fade_in = elapsed.clamp(0.0, 1.0);
        let fade_out = self.timer.clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }

    pub fn update(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
        self.amount = self.intensity();
    }
}

pub fn blind_vision_radius(intensity: f32, width: f32, height: f32) -> f32 {
    let full_radius = (width * width + height * height).sqrt() * 0.5;
    full_radius + (0.1 * height - full_radius) * intensity.clamp(0.0, 1.0)
}

#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Clone, Copy)]
pub struct BlindUniform {
    pub amount: f32,
    pub aspect_ratio: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(duration: f32) -> BlindEffect {
        BlindEffect {
            id: prism::PostProcessPassId(0), timer: duration,
            total_duration: duration, amount: 0.0, aspect_ratio: 1.0,
        }
    }

    #[test]
    fn blind_fades_in_holds_then_fades_out_during_last_second() {
        let mut blind = effect(3.0);
        blind.update(0.0);
        assert_eq!(blind.amount, 0.0);
        blind.update(0.5);
        assert_eq!(blind.amount, 0.5);
        blind.update(0.5);
        assert_eq!(blind.amount, 1.0);
        blind.update(0.5);
        assert_eq!(blind.amount, 1.0);
        blind.update(0.5);
        assert_eq!(blind.amount, 1.0);
        blind.update(0.5);
        assert_eq!(blind.amount, 0.5);
        blind.update(1.0);
        assert_eq!(blind.amount, 0.0);
        assert_eq!(blind.timer, 0.0);
    }

    #[test]
    fn short_blind_has_overlapping_fades_and_zero_duration_is_inactive() {
        let mut blind = effect(0.5);
        blind.update(0.0);
        assert_eq!(blind.amount, 0.0);
        blind.update(0.25);
        assert_eq!(blind.amount, 0.25);
        blind.update(0.25);
        assert_eq!(blind.amount, 0.0);
        assert_eq!(effect(0.0).intensity(), 0.0);
    }

    #[test]
    fn vision_radius_tracks_shader_and_reveals_the_full_view_at_expiration() {
        assert_eq!(blind_vision_radius(1.0, 800.0, 600.0), 60.0);
        assert_eq!(blind_vision_radius(0.5, 800.0, 600.0), 280.0);
        assert_eq!(blind_vision_radius(0.0, 800.0, 600.0), 500.0);
    }
}

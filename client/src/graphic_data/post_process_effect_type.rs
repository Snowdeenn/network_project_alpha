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


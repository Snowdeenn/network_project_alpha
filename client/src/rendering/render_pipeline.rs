// src/renderer/pipeline.rs

use raylib::prelude::*;
use crate::rendering::shader_manager::{PassKind, ShaderManager};

pub struct RenderPipeline {
    post_process_target: RenderTexture2D,
}

#[allow(dead_code)]
impl RenderPipeline {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread, width: i32, height: i32) -> Self {
        let target = rl
            .load_render_texture(thread, width as u32, height as u32)
            .expect("[RenderPipeline] Échec création RenderTexture2D post-process");

        Self {
            post_process_target: target,
        }
    }

    pub fn resize(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, width: i32, height: i32) {
        self.post_process_target = rl
            .load_render_texture(thread, width as u32, height as u32)
            .expect("[RenderPipeline] Échec du redimensionnement de la target");
    }

    /// Orchestrateur principal : orchestre le flux d'exécution passe par passe
    pub fn execute<FWorld, FVfx, FHud>(
        &mut self,
        d: &mut RaylibDrawHandle,
        thread: &RaylibThread,
        shaders: &mut ShaderManager,
        render_world: FWorld,
        render_vfx: FVfx,
        render_hud: FHud,
    ) where
        FWorld: FnOnce(&mut RaylibTextureMode<RaylibDrawHandle>),
        FVfx: FnOnce(&mut RaylibTextureMode<RaylibDrawHandle>),
        FHud: FnOnce(&mut RaylibDrawHandle),
    {
        {
            let mut draw_target = d.begin_texture_mode(thread, &mut self.post_process_target);
            draw_target.clear_background(Color::BLACK);

            Self::pass_world(&mut draw_target, shaders, render_world);
            Self::pass_vfx(&mut draw_target, shaders, render_vfx);
        }
        Self::pass_post_process(d, &self.post_process_target, shaders);
        Self::pass_hud(d, shaders, render_hud);
    }

    fn pass_world<F>(
        draw_target: &mut RaylibTextureMode<RaylibDrawHandle>,
        shaders: &mut ShaderManager,
        render_world: F,
    ) where
        F: FnOnce(&mut RaylibTextureMode<RaylibDrawHandle>),
    {
        if let Some(shader) = shaders.get_pass_shader_mut(PassKind::World) {
            let mut mode = draw_target.begin_shader_mode(shader);
            render_world(&mut mode);
        } else {
            render_world(draw_target);
        }
    }

    fn pass_vfx<F>(
        draw_target: &mut RaylibTextureMode<RaylibDrawHandle>,
        shaders: &mut ShaderManager,
        render_vfx: F,
    ) where
        F: FnOnce(&mut RaylibTextureMode<RaylibDrawHandle>),
    {
        if let Some(shader) = shaders.get_pass_shader_mut(PassKind::Vfx) {
            let mut mode = draw_target.begin_shader_mode(shader);
            render_vfx(&mut mode);
        } else {
            render_vfx(draw_target);
        }
    }

    fn pass_post_process(
        d: &mut RaylibDrawHandle,
        target: &RenderTexture2D,
        shaders: &mut ShaderManager,
    ) {
        // En Raylib/OpenGL, les coordonnées Y d'une RenderTarget sont inversées (d'où le -height)
        let source_rec = Rectangle::new(
            0.0,
            0.0,
            target.texture().width() as f32,
            -target.texture().height() as f32,
        );

        if let Some(shader) = shaders.get_pass_shader_mut(PassKind::PostProcess) {
            let mut mode = d.begin_shader_mode(shader);
            mode.draw_texture_rec(target.texture(), source_rec, Vector2::zero(), Color::WHITE);
        } else {
            d.draw_texture_rec(target.texture(), source_rec, Vector2::zero(), Color::WHITE);
        }
    }

    fn pass_hud<F>(
        d: &mut RaylibDrawHandle,
        shaders: &mut ShaderManager,
        render_hud: F,
    ) where
        F: FnOnce(&mut RaylibDrawHandle),
    {
        if let Some(shader) = shaders.get_pass_shader_mut(PassKind::Hud) {
            let mut mode = d.begin_shader_mode(shader);
            render_hud(&mut mode);
        } else {
            render_hud(d);
        }
    }
}
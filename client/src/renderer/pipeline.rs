use std::collections::HashMap;

use super::{
    Renderer, debug_ui,render_world,
    types::{FrameState, RenderContext},
};
use crate::TICK_DURATION;
use crate::config::*;
use crate::event::ClientState;
use crate::event::GamePhase;
use crate::particle::ParticleSystem;
use crate::renderer::ScreenScale;
use crate::renderer::animation::AnimEntity;
use crate::renderer::texture::TextureManager;
use raylib::prelude::*;

impl Renderer {
    pub(super) fn render_game_world(
        d: &mut RaylibDrawHandle,
        frame: &FrameState,
        client_state: &ClientState,
        particle_system: &ParticleSystem,
        dt: f32,
        cam: Camera2D,
        screen_scale: &ScreenScale,
        texture: &TextureManager, // ajuste le chemin si besoin
        anim_entities: &mut HashMap<u64, AnimEntity>, // ajuste selon ton type réel
    ) {
        let mut d2 = d.begin_mode2D(cam);
        let t = (frame.last_snap_time.elapsed().as_secs_f32() / TICK_DURATION.as_secs_f32())
            .clamp(0.0, 1.0);
        let s = screen_scale;

        match frame.current {
            None => {
                d2.draw_text("Connexion...", -80, -10, 20, Color::WHITE);
            }
            Some(curr) => match client_state.phase {
                GamePhase::Dead => {
                    d2.draw_text(
                        " YOU'RE DEAD",
                        s.x(750.0 / 1920.0),
                        s.y(500.0 / 1080.0),
                        s.font(120.0 / 1920.0),
                        Color::RED,
                    );
                }
                _ => render_world(
                    &mut d2,
                    particle_system,
                    texture,
                    anim_entities,
                    frame.prev,
                    curr,
                    t,
                    dt,
                ),
            },
        }
    }

    pub(super) fn render_game_hud(
        d: &mut RaylibDrawHandle,
        client_state: &ClientState,
        screen_scale: &ScreenScale,
    ) {
        let s = screen_scale;
        if let GamePhase::BetweenWave { time_remaining, .. } = client_state.phase {
            let remaining = format!(
                " Temps avant la prochaine vague {}s",
                time_remaining.as_secs()
            );
            d.draw_text(
                &remaining,
                s.x(WAVE_TIMER_X),
                s.y(WAVE_TIMER_Y),
                s.font(WAVE_TIMER_FONT),
                Color::RED,
            );

            if client_state.phase.can_show_shop() {
                d.draw_text(
                    "Shop disponible — appuie sur G",
                    s.x(HUD_SHOP_NOTIF_X),
                    s.y(HUD_SHOP_NOTIF_Y),
                    s.font(HUD_SHOP_NOTIF_FONT),
                    Color::GOLD,
                );
            }
        }

        //hud::render_shop(d, client_state, s);
    }

    pub(super) fn render_ui_frameworks(
        d: &mut RaylibDrawHandle,
        ctx: &mut RenderContext,
        ui: &mut imgui::Ui,
        client_state: &mut ClientState,
        cam: &Camera2D,
    ) {
        ctx.ui_ctx.collect(ctx.buffer);
        ctx.buffer.sort();
        ctx.buffer.flush(d, ctx.tex_registry, ctx.shader_registry);
        ctx.buffer.clear();

        debug_ui::process_debug(ui, d, cam, client_state);
    }
}

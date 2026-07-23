use shared::protocol::StateSnapshot;
use std::time::Instant;
use ui::{context::UiContext, draw::DrawCommandBuffer};

use crate::renderer::{shader_manager::ShaderManager, texture_manager::TextureManager};

/// Regroupe l'état réseau nécessaire à l'interpolation d'une frame
pub struct FrameState<'a> {
    pub prev: Option<&'a StateSnapshot>,
    pub current: Option<&'a StateSnapshot>,
    pub last_snap_time: Instant,
}

/// Regroupe temporairement les outils de ton Framework UI pour le rendu
pub struct RenderContext<'a> {
    pub buffer: &'a mut DrawCommandBuffer,
    pub texture_manager: &'a TextureManager,
    pub shader_manager: &'a mut ShaderManager,
    pub ui_ctx: &'a mut UiContext,
}

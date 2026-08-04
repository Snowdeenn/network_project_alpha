use utils::protocol::StateSnapshot;
use std::time::Instant;
use ui::{context::UiContext, draw::DrawCommandBuffer};

/// Regroupe l'état réseau nécessaire à l'interpolation d'une frame
pub struct FrameState<'a> {
    pub prev: Option<&'a StateSnapshot>,
    pub current: Option<&'a StateSnapshot>,
    pub last_snap_time: Instant,
}

/// Regroupe temporairement les outils de ton Framework UI pour le rendu
pub struct RenderContext<'a> {
    pub buffer: &'a mut DrawCommandBuffer,
    pub shader_manager: &'a mut prism::ShaderManager,
    pub ui_ctx: &'a mut UiContext,
}

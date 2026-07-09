use std::time::Instant;
use shared::protocol::StateSnapshot;
use ui::{context::UiContext, draw::DrawCommandBuffer, shader::ShaderRegistry, texture::TextureRegistry};

/// Regroupe l'état réseau nécessaire à l'interpolation d'une frame
pub struct FrameState<'a> {
    pub prev: Option<&'a StateSnapshot>,
    pub current: Option<&'a StateSnapshot>,
    pub last_snap_time: Instant,
}

/// Regroupe temporairement les outils de ton Framework UI pour le rendu
pub struct RenderContext<'a> {
    pub buffer: &'a mut DrawCommandBuffer,
    pub tex_registry: &'a TextureRegistry,
    pub shader_registry: &'a mut ShaderRegistry,
    pub ui_ctx: &'a mut UiContext,
}
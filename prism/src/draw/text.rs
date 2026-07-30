use crate::{context::GpuContext, draw::commands::DrawCommand, errors::TextRendererError};

pub struct TextRenderer {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    renderer: glyphon::TextRenderer,
}

impl TextRenderer {
    pub fn new(ctx: &GpuContext, surface_format: wgpu::TextureFormat) -> Self {
        let font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = &glyphon::Cache::new(&ctx.device);
        let viewport = glyphon::Viewport::new(&ctx.device, cache);
        let mut atlas = glyphon::TextAtlas::new(&ctx.device, &ctx.queue, cache, surface_format);
        let renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );
        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderer,
        }
    }

    pub fn prepare(
        &mut self,
        ctx: &GpuContext,
        commands: &[DrawCommand],
    ) -> Result<(), TextRendererError> {
        self.viewport.update(
            &ctx.queue,
            glyphon::Resolution {
                width: ctx.size.width,
                height: ctx.size.height,
            },
        );

        let text_areas = commands.iter().filter_map(|cmd| match cmd {
            DrawCommand::Text {
                content,
                pos,
                color,
                ..
            } => Some(glyphon::TextArea {
                buffer: content,
                left: pos[0],
                top: pos[1],
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: ctx.size.width as i32,
                    bottom: ctx.size.width as i32,
                },
                default_color: glyphon::Color::rgba(
                    color[0] as u8,
                    color[1] as u8,
                    color[2] as u8,
                    color[3] as u8,
                ),
                custom_glyphs: &[],
            }),
            _ => None,
        });

        self.renderer.prepare(
            &ctx.device,
            &ctx.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;
        Ok(())
    }

    pub fn render<'a>(&mut self, pass: &mut wgpu::RenderPass<'a>) -> Result<(), TextRendererError> {
        self.renderer.render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
    }
}

//! Glyphon text overlay, independent from Prism's historical HUD implementation.
use super::Primitive;
use std::collections::{HashMap, HashSet};

pub(crate) struct TextOverlay {
    fonts: glyphon::FontSystem,
    swash: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    renderer: glyphon::TextRenderer,
    buffers: HashMap<(String, u32, u32, u32), glyphon::Buffer>,
}

impl TextOverlay {
    pub fn new(context: &prism::GpuContext) -> Self {
        let cache = glyphon::Cache::new(&context.device);
        let viewport = glyphon::Viewport::new(&context.device, &cache);
        let mut atlas = glyphon::TextAtlas::new(
            &context.device,
            &context.queue,
            &cache,
            context.surface_format(),
        );
        let renderer =
            glyphon::TextRenderer::new(&mut atlas, &context.device, Default::default(), None);
        Self {
            fonts: glyphon::FontSystem::new(),
            swash: glyphon::SwashCache::new(),
            viewport,
            atlas,
            renderer,
            buffers: HashMap::new(),
        }
    }

    pub fn render(
        &mut self,
        context: &prism::GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        [width, height]: [u32; 2],
        commands: &[Primitive],
    ) -> prism::RenderResult<()> {
        let texts: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                Primitive::Text {
                    content,
                    pos,
                    size,
                    color,
                    ..
                } => Some((content, pos, size, color)),
                _ => None,
            })
            .collect();
        if texts.is_empty() {
            self.buffers.clear();
            return Ok(());
        }
        self.viewport
            .update(&context.queue, glyphon::Resolution { width, height });
        let mut active = HashSet::new();
        for (content, _, size, _) in &texts {
            let key = ((*content).clone(), size.to_bits(), width, height);
            self.buffers.entry(key.clone()).or_insert_with(|| {
                let mut buffer = glyphon::Buffer::new(
                    &mut self.fonts,
                    glyphon::Metrics::new(**size, **size * 1.2),
                );
                buffer.set_size(Some(width as f32), Some(height as f32));
                buffer.set_text(
                    content,
                    &glyphon::Attrs::new(),
                    glyphon::Shaping::Advanced,
                    Some(glyphon::cosmic_text::Align::Left),
                );
                buffer.shape_until_scroll(&mut self.fonts, true);
                buffer
            });
            active.insert(key);
        }
        self.buffers.retain(|key, _| active.contains(key));
        let areas = texts
            .iter()
            .map(|(content, pos, size, color)| glyphon::TextArea {
                buffer: &self.buffers[&((*content).clone(), size.to_bits(), width, height)],
                left: pos[0],
                top: pos[1],
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                },
                default_color: glyphon::Color::rgba(
                    (color[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (color[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (color[2] * 255.0).clamp(0.0, 255.0) as u8,
                    (color[3] * 255.0).clamp(0.0, 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        self.renderer
            .prepare(
                &context.device,
                &context.queue,
                &mut self.fonts,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash,
            )
            .map_err(|error| prism::RenderError::new("text preparation", error.to_string()))?;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HUD text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        self.renderer
            .render(&self.atlas, &self.viewport, &mut pass)
            .map_err(|error| prism::RenderError::new("text rendering", error.to_string()))?;
        drop(pass);
        self.atlas.trim();
        Ok(())
    }
}

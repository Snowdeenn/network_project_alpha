use crate::{context::GpuContext, draw::commands::DrawCommand, errors::TextRendererError};

use std::collections::HashMap;

pub struct TextBufferCache {
    font_system: glyphon::FontSystem,
    buffers: HashMap<(String, u32), glyphon::Buffer>,
}

impl TextBufferCache {
    pub fn new() -> Self {
        Self {
            font_system: glyphon::FontSystem::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, text: &str, size: f32) -> &glyphon::Buffer {
        let key = (text.to_string(), size.to_bits());

        if !self.buffers.contains_key(&key) {
            let metrics = glyphon::Metrics::new(size, size * 1.2);
            let mut buffer = glyphon::Buffer::new(&mut self.font_system, metrics);
            buffer.set_text(
                text,
                &glyphon::Attrs::new().family(glyphon::Family::SansSerif),
                glyphon::Shaping::Advanced,
                Some(glyphon::cosmic_text::Align::Center),
            );
            self.buffers.insert(key.clone(), buffer);
        }

        self.buffers.get(&key).unwrap()
    }

    pub fn font_system_mut(&mut self) -> &mut glyphon::FontSystem {
        &mut self.font_system
    }

    pub fn trim(&mut self) {
        // Vide le cache — à appeler quand le contenu change fréquemment
        // Pour l'instant garde tout en mémoire
    }
}

pub struct TextRenderer {
    cache_text: TextBufferCache,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    renderer: glyphon::TextRenderer,
}

impl TextRenderer {
    pub fn new(ctx: &GpuContext, surface_format: wgpu::TextureFormat) -> Self {
        let cache_text = TextBufferCache::new();
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
            cache_text,
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
        let text_commands: Vec<(&str, [f32; 2], f32, [f32; 4])> = commands
            .iter()
            .filter_map(|cmd| match cmd {
                DrawCommand::Text {
                    content,
                    pos,
                    size,
                    color,
                    ..
                } => Some((content.as_str(), *pos, *size, *color)),
                _ => None,
            })
            .collect();
        for (content, _, size, _) in &text_commands {
            self.cache_text.get_or_create(content, *size);
        }
        let mut text_areas = Vec::new();
        for (content, pos, size, color) in &text_commands {
            let key = (content.to_string(), size.to_bits());
            let buffer = self.cache_text.buffers.get(&key).unwrap();
            text_areas.push(glyphon::TextArea {
                buffer,
                left: pos[0],
                top: pos[1],
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: ctx.size.width as i32,
                    bottom: ctx.size.height as i32,
                },
                default_color: glyphon::Color::rgba(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    (color[3] * 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        }

        self.renderer.prepare(
            &ctx.device,
            &ctx.queue,
            &mut self.cache_text.font_system,
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

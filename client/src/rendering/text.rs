//! Glyphon text overlay, independent from Prism's historical HUD implementation.
use super::Primitive;
use std::collections::{HashMap, HashSet};

pub(crate) struct DeviceText {
    fonts: glyphon::FontSystem,
    swash: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    renderer: glyphon::TextRenderer,
    buffers: HashMap<(String, u32, u32, u32), glyphon::Buffer>,
}

impl DeviceText {
    pub fn new(context: &crate::rendering::GpuContext) -> Self {
        let cache = glyphon::Cache::new(&context.device());
        let viewport = glyphon::Viewport::new(&context.device(), &cache);
        let mut atlas = glyphon::TextAtlas::new(
            &context.device(),
            &context.queue(),
            &cache,
            context.surface_format(),
        );
        let renderer =
            glyphon::TextRenderer::new(&mut atlas, &context.device(), Default::default(), None);
        Self {
            fonts: glyphon::FontSystem::new(),
            swash: glyphon::SwashCache::new(),
            viewport,
            atlas,
            renderer,
            buffers: HashMap::new(),
        }
    }

    pub fn prepare(
        &mut self,
        context: &crate::rendering::GpuContext,
        [width, height]: [u32; 2],
        commands: &[Primitive],
    ) -> velum::backend::RenderResult<bool> {
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
            return Ok(false);
        }
        self.viewport
            .update(&context.queue(), glyphon::Resolution { width, height });
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
                &context.device(),
                &context.queue(),
                &mut self.fonts,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash,
            )
            .map_err(|error| {
                velum::backend::RenderError::new("text preparation", error.to_string())
            })?;
        Ok(true)
    }
}

use velum::backend::text;

pub(crate) enum TextOverlay {
    Window(HashMap<(String, u32, u32), velum::OwnedResource<text::TextId>>),
    Device(DeviceText),
}
impl TextOverlay {
    pub fn new(context: &super::GpuContext) -> Self {
        if context.context_id().is_none() {
            Self::Window(Default::default())
        } else {
            Self::Device(DeviceText::new(context))
        }
    }
    pub fn prepare(
        &mut self,
        context: &super::GpuContext,
        size: [u32; 2],
        commands: &[Primitive],
    ) -> velum::backend::RenderResult<bool> {
        let cache = match self {
            Self::Device(text) => return text.prepare(context, size, commands),
            Self::Window(cache) => cache,
        };
        let mut visible = HashSet::new();
        let mut instances = Vec::new();
        for cmd in commands {
            let Primitive::Text {
                content,
                pos,
                size: font_size,
                color,
                ..
            } = cmd
            else {
                continue;
            };
            let key = (content.clone(), font_size.to_bits(), size[0]);
            visible.insert(key.clone());
            let owner = match cache.entry(key) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => e.insert(
                    text::register(text::TextDescription {
                        content,
                        font_size: *font_size,
                        line_height: *font_size * 1.2,
                        max_width: Some(size[0] as f32),
                        alignment: Some(text::Align::Left),
                        ..Default::default()
                    })
                    .map_err(|e| velum::backend::RenderError::new("HUD text", e.to_string()))?,
                ),
            };
            let mut item = text::Text::new(owner.id(), text::TextSpace::Screen);
            item.position = *pos;
            item.color = *color;
            instances.push(item);
        }
        cache.retain(|key, _| visible.contains(key));
        if instances.is_empty() {
            return Ok(false);
        }
        text::prepare(
            &instances,
            &text::TextView {
                resolution: size,
                scale_factor: 1.0,
                camera: &velum::camera::Camera2D::new(),
            },
        )
        .map_err(|e| velum::backend::RenderError::new("HUD text", e.to_string()))?;
        Ok(true)
    }
}
pub(super) struct HudText(pub velum::RenderDataId<TextOverlay>);

impl velum::backend::pass::RenderOperation for HudText {
    fn execute(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        data: &mut velum::RenderData,
    ) -> Result<(), velum::backend::pass::PassError> {
        let overlay = data
            .require_mut(self.0)
            .map_err(velum::backend::pass::PassError::Data)?;
        let TextOverlay::Device(overlay) = overlay else {
            return text::draw_all(pass).map_err(Into::into);
        };
        overlay
            .renderer
            .render(&overlay.atlas, &overlay.viewport, pass)
            .map_err(|error| {
                velum::backend::RenderError::new("text rendering", error.to_string())
            })?;
        overlay.atlas.trim();
        Ok(())
    }
}

//! The only command-level V1 boundary, until Nodus exposes a V2/iterable draw buffer.
use super::{BlendMode, Primitive, SceneFrame, TextureId};
use velum::backend::{RenderError, RenderResult, material::MaterialTemplateId};
use velum::{RenderData, RenderDataId, SharedResource};

/// Historical Nodus tokens mapped to native resources; owns no GPU resources.
#[derive(Default)]
pub struct NodusBridge {
    nodus_materials: prism_legacy::arena::Arena<
        RenderDataId<(
            SharedResource<MaterialTemplateId>,
            super::shader_bindings::MeshBindings,
        )>,
        prism_legacy::ids::MaterialTag,
    >,
    nodus_textures: prism_legacy::arena::Arena<TextureId, prism_legacy::ids::TextureTag>,
}

pub fn collect(
    buffer: &nodus::DrawCommandBuffer,
    bridge: &NodusBridge,
    data: &velum::RenderData,
    frame: &mut SceneFrame,
) {
    let mut ui = prism_legacy::Frame::new();
    buffer.collect_into(&mut ui);
    for command in ui.hud_commands().commands() {
        match convert(command, bridge, data) {
            Ok(command) => frame.push_hud(command),
            Err(error) => tracing::error!(%error, "Commande Nodus ignorée"),
        }
    }
}

fn convert(
    command: &prism_legacy::DrawCommand,
    bridge: &NodusBridge,
    data: &velum::RenderData,
) -> velum::backend::RenderResult<Primitive> {
    use prism_legacy::DrawCommand as Ui;
    let uv = |uv: &Option<prism_legacy::UvRect>| {
        uv.map(|r| velum::geometry::UvRect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        })
    };
    let blend = |mode: &prism_legacy::BlendMode| match mode {
        prism_legacy::BlendMode::Additive => BlendMode::Additive,
        _ => BlendMode::Alpha,
    };
    Ok(match command {
        Ui::Shape {
            shape:
                prism_legacy::Shape::Quad {
                    pos,
                    size,
                    rotation,
                    color,
                    uv: rect,
                },
            blend: mode,
            layer,
        } => Primitive::Shape {
            shape: velum::geometry::Shape::Quad {
                pos: *pos,
                size: *size,
                rotation: *rotation,
                color: *color,
                uv: uv(rect),
            },
            blend: blend(mode),
            layer: *layer,
        },
        Ui::Texture {
            id,
            pos,
            size,
            rotation,
            uv: rect,
            tint,
            blend: mode,
            layer,
        } => Primitive::Texture {
            flash: false,
            id: bridge.nodus_texture(*id)?,
            pos: *pos,
            size: *size,
            rotation: *rotation,
            uv: uv(rect),
            tint: *tint,
            blend: blend(mode),
            layer: *layer,
        },
        Ui::Material {
            material_id,
            texture_id,
            pos,
            size,
            rotation,
            uv: rect,
            tint,
            uniform_data,
            blend: mode,
            layer,
        } => Primitive::Material {
            template: bridge
                .nodus_materials
                .get(*material_id)
                .and_then(|id| data.get(*id))
                .map(|(template, _)| template.clone())
                .ok_or_else(|| {
                    velum::backend::RenderError::new("Nodus material", "unregistered UI token")
                })?,
            layout: data
                .require(
                    *bridge
                        .nodus_materials
                        .get(*material_id)
                        .ok_or_else(|| RenderError::new("Nodus material", "unregistered token"))?,
                )
                .map_err(|e| RenderError::new("Nodus material", e.to_string()))?
                .1,
            resource_bindings: Default::default(),
            texture_id: texture_id.map(|id| bridge.nodus_texture(id)).transpose()?,
            pos: *pos,
            size: *size,
            rotation: *rotation,
            uv: uv(rect),
            tint: *tint,
            uniform_data: uniform_data.clone(),
            blend: blend(mode),
            layer: *layer,
        },
        Ui::NinePatch {
            id,
            pos,
            size,
            texture_size: _,
            margins,
            tint,
            blend: mode,
            layer,
        } => {
            let id = bridge.nodus_texture(*id)?;
            let (width, height) = super::texture::size(id)?;
            Primitive::NinePatch {
                id,
                pos: *pos,
                size: *size,
                texture_size: [width as f32, height as f32],
                margins: velum::geometry::NinePatchMargins {
                    top: margins.top,
                    bottom: margins.bottom,
                    left: margins.left,
                    right: margins.right,
                },
                tint: *tint,
                blend: blend(mode),
                layer: *layer,
            }
        }
        Ui::Text {
            content,
            pos,
            size,
            color,
            layer,
        } => Primitive::Text {
            content: content.clone(),
            pos: *pos,
            size: *size,
            color: *color,
            layer: *layer,
        },
        _ => {
            return Err(velum::backend::RenderError::new(
                "Nodus draw",
                "unsupported UI primitive",
            ));
        }
    })
}

impl NodusBridge {
    pub fn register_material(
        &mut self,
        data: &mut RenderData,
        template: SharedResource<MaterialTemplateId>,
        layout: super::shader_bindings::MeshBindings,
    ) -> prism_legacy::ids::MaterialId {
        self.nodus_materials.insert(data.insert((template, layout)))
    }
    pub fn register_nodus_texture(
        &mut self,
        id: TextureId,
    ) -> RenderResult<prism_legacy::ids::TextureId> {
        super::texture::size(id)?;
        Ok(self.nodus_textures.insert(id))
    }
    pub(crate) fn nodus_texture(
        &self,
        id: prism_legacy::ids::TextureId,
    ) -> RenderResult<TextureId> {
        let texture = *self
            .nodus_textures
            .get(id)
            .ok_or_else(|| RenderError::new("Nodus texture", "unregistered token"))?;
        super::texture::size(texture)?;
        Ok(texture)
    }
    pub fn unregister_nodus_texture(&mut self, id: prism_legacy::ids::TextureId) -> bool {
        self.nodus_textures.remove(id).is_some()
    }
}

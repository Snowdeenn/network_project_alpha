//! The only command-level V1 boundary, until Nodus exposes a V2/iterable draw buffer.
use super::{BlendMode, Primitive, RenderAssets, SceneFrame};

pub fn collect(buffer: &nodus::DrawCommandBuffer, assets: &RenderAssets, frame: &mut SceneFrame) {
    let mut ui = prism_legacy::Frame::new();
    buffer.collect_into(&mut ui);
    for command in ui.hud_commands().commands() {
        match convert(command, assets) {
            Ok(command) => frame.push_hud(command),
            Err(error) => tracing::error!(%error, "Commande Nodus ignorée"),
        }
    }
}

fn convert(
    command: &prism_legacy::DrawCommand,
    assets: &RenderAssets,
) -> prism::RenderResult<Primitive> {
    use prism_legacy::DrawCommand as Ui;
    let uv = |uv: &Option<prism_legacy::UvRect>| {
        uv.map(|r| prism::UvRect {
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
            shape: prism::Shape::Quad {
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
            id: assets.nodus_texture(*id)?,
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
            template: assets
                .nodus_materials
                .get(*material_id)
                .copied()
                .ok_or_else(|| {
                    prism::RenderError::new("Nodus material", "unregistered UI token")
                })?,
            texture_id: texture_id.map(|id| assets.nodus_texture(id)).transpose()?,
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
            let id = assets.nodus_texture(*id)?;
            let (width, height) = assets.texture_size(id)?;
            Primitive::NinePatch {
                id,
                pos: *pos,
                size: *size,
                texture_size: [width as f32, height as f32],
                margins: prism::NinePatchMargins {
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
            return Err(prism::RenderError::new(
                "Nodus draw",
                "unsupported UI primitive",
            ));
        }
    })
}

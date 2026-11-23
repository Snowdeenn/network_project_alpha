//! Texture loading for the explicit Alpha GPU context.
use super::{BlendMode, TextureId, shader_bindings::HEALTH_BINDINGS};
use std::path::Path;
use velum::{
    SharedResource,
    backend::{
        self as gpu, BindingSchema, RenderError, RenderResult,
        context::ContextId,
        material::{MaterialTemplate, MaterialTemplateId, ShaderEntry},
        texture,
    },
};

pub(crate) fn load_texture(
    context: Option<ContextId>,
    path: &Path,
) -> Result<wgpu::Texture, image::ImageError> {
    let image = image::open(path)?.into_rgba8();
    let (width, height) = image.dimensions();
    // Velum's file loader uses Rgba8Unorm; keep sRGB decoding for Alpha assets.
    let mut builder = texture::TextureBuilder::new()
        .with_size(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        })
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .with_view_formats(&[]);
    if let Some(label) = path.to_str() {
        builder = builder.with_label(label);
    }
    let raw = builder.build_in(context);
    gpu::context::queue(context).write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &raw,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        raw.size(),
    );
    Ok(raw)
}

pub fn create_health_material(
    context: &super::GpuContext,
) -> RenderResult<SharedResource<MaterialTemplateId>> {
    let vertex = gpu::try_shader(
        Some(context.velum_context_id()),
        "HUD vertex",
        include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
    )?;
    let fragment = gpu::try_shader(
        Some(context.velum_context_id()),
        "health",
        include_str!("../graphic_data/shader/progress_bar.frag.wgsl"),
    )?;
    mesh_template(
        Some(context.velum_context_id()),
        "health",
        vertex,
        fragment,
        context.surface_format(),
        BlendMode::Alpha,
        HEALTH_BINDINGS.schema()?,
    )
}

pub(crate) fn mesh_template(
    context: Option<ContextId>,
    label: &str,
    vertex: wgpu::ShaderModule,
    fragment: wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: BlendMode,
    schema: BindingSchema,
) -> RenderResult<SharedResource<MaterialTemplateId>> {
    let mut template = MaterialTemplate::fullscreen(context, label, vertex, format);
    template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
    template.schema = schema;
    template.vertex_layouts = vec![Some(velum::geometry::Vertex::layout().into())];
    template.targets[0].as_mut().unwrap().blend = Some(match blend {
        BlendMode::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        BlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        },
    });
    velum::try_material_template!(template).map(velum::OwnedResource::share)
}
pub(crate) fn texture_size(id: TextureId) -> RenderResult<(u32, u32)> {
    let resource =
        texture::get(id).ok_or_else(|| RenderError::new("texture", "missing texture"))?;
    let texture = match resource.as_ref() {
        texture::TextureResource::Separate(resource)
        | texture::TextureResource::Array(resource)
        | texture::TextureResource::Atlas { resource, .. } => &resource.texture,
    };
    Ok((texture.as_raw().width(), texture.as_raw().height()))
}

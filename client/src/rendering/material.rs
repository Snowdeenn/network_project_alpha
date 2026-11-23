//! Alpha mesh material templates.
use super::{BlendMode, shader_bindings::HEALTH_BINDINGS};
use velum::{
    SharedResource,
    backend::{
        self as gpu, BindingSchema, RenderResult,
        context::ContextId,
        material::{MaterialTemplate, MaterialTemplateId, ShaderEntry},
    },
};

pub fn create_health_material(
    context: &super::GpuContext,
) -> RenderResult<SharedResource<MaterialTemplateId>> {
    let vertex = gpu::try_shader(
        context.context_id(),
        "HUD vertex",
        include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
    )?;
    let fragment = gpu::try_shader(
        context.context_id(),
        "health",
        include_str!("../graphic_data/shader/progress_bar.frag.wgsl"),
    )?;
    mesh_template(
        context.context_id(),
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
    template.targets[0].as_mut().unwrap().blend = Some(blend_state(blend));
    velum::try_material_template!(template).map(velum::OwnedResource::share)
}

pub(super) fn blend_state(blend: BlendMode) -> wgpu::BlendState {
    match blend {
        BlendMode::Alpha => wgpu::BlendState::ALPHA_BLENDING,
        BlendMode::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        },
    }
}

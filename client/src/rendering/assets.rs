//! Scene assets held by Velum owners. All GPU resources use Velum's native API.
use super::{BlendMode, TextureId};
use gpu::{BindingSchema, GroupSchema, RenderError, RenderResult};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use velum::{
    SharedResource,
    backend::{
        self as gpu,
        bindings::{self, BindingResource, BindingSet},
        context::ContextId,
        material::{self, MaterialId, MaterialTemplate, MaterialTemplateId, ShaderEntry},
        sampler, texture,
    },
};

pub struct RenderAssets {
    pub context: Option<ContextId>,
    textures: HashMap<PathBuf, SharedResource<TextureId>>,
    texture_bindings: HashMap<TextureId, SharedResource<gpu::BindingSetId>>,
    sprite_materials: HashMap<(TextureId, BlendMode, bool), SharedResource<MaterialId>>,
    pub(crate) shapes: HashMap<BlendMode, SharedResource<MaterialId>>,
    sprite_templates: HashMap<(BlendMode, bool), SharedResource<MaterialTemplateId>>,
    pub(crate) sampler: sampler::Sampler<sampler::Filtering>,
    pub(crate) nodus_materials: prism_legacy::arena::Arena<
        SharedResource<MaterialTemplateId>,
        prism_legacy::ids::MaterialTag,
    >,
    nodus_textures: prism_legacy::arena::Arena<TextureId, prism_legacy::ids::TextureTag>,
}

pub(crate) fn uniform_group(index: u32, size: u64, visibility: wgpu::ShaderStages) -> GroupSchema {
    GroupSchema {
        index,
        entries: vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size),
            },
            count: None,
        }],
    }
}
pub(crate) fn texture_group(index: u32) -> GroupSchema {
    GroupSchema {
        index,
        entries: vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    }
}
pub(crate) fn mesh_template(
    context: Option<ContextId>,
    label: &str,
    vertex: wgpu::ShaderModule,
    fragment: wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    blend: BlendMode,
    groups: Vec<GroupSchema>,
) -> MaterialTemplate {
    let mut template = MaterialTemplate::fullscreen(context, label, vertex, format);
    template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
    template.schema = BindingSchema { groups };
    template.vertex_layouts = vec![Some(velum::geometry::mesh::Vertex::layout().into())];
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
    template
}
impl RenderAssets {
    pub fn new(context: &super::GpuContext) -> RenderResult<Self> {
        let ctx = Some(context.velum_context_id());
        let vertex = gpu::try_shader(
            ctx,
            "scene vertex",
            include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
        )?;
        let flat = gpu::try_shader(
            ctx,
            "scene color",
            include_str!("../graphic_data/shader/default.frag.wgsl"),
        )?;
        let textured = gpu::try_shader(
            ctx,
            "scene texture",
            include_str!("../graphic_data/shader/default_textured.frag.wgsl"),
        )?;
        let flash = gpu::try_shader(
            ctx,
            "entity tint",
            include_str!("../graphic_data/shader/entity_flash.frag.wgsl"),
        )?;
        let mut shapes = HashMap::new();
        let mut sprite_templates = HashMap::new();
        for blend in [BlendMode::Alpha, BlendMode::Additive] {
            let camera = uniform_group(0, 64, wgpu::ShaderStages::VERTEX);
            let template = material::try_template(mesh_template(
                ctx,
                "shape",
                vertex.clone(),
                flat.clone(),
                context.surface_format(),
                blend,
                vec![camera.clone()],
            ))?
            .share();
            shapes.insert(
                blend,
                material::try_register(material::Material {
                    template,
                    bindings: vec![],
                })?
                .share(),
            );
            for (is_flash, fragment) in [(false, &textured), (true, &flash)] {
                let template = material::try_template(mesh_template(
                    ctx,
                    "sprite",
                    vertex.clone(),
                    fragment.clone(),
                    context.surface_format(),
                    blend,
                    vec![camera.clone(), texture_group(1)],
                ))?
                .share();
                sprite_templates.insert((blend, is_flash), template);
            }
        }
        Ok(Self {
            context: ctx,
            textures: HashMap::new(),
            texture_bindings: HashMap::new(),
            sprite_materials: HashMap::new(),
            shapes,
            sprite_templates,
            sampler: sampler::pixel_art_in(ctx, "pixel art sampler"),
            nodus_materials: Default::default(),
            nodus_textures: Default::default(),
        })
    }
    pub fn create_health_material(
        &mut self,
        context: &super::GpuContext,
    ) -> RenderResult<prism_legacy::ids::MaterialId> {
        let vertex = gpu::try_shader(
            self.context,
            "HUD vertex",
            include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
        )?;
        let fragment = gpu::try_shader(
            self.context,
            "health",
            include_str!("../graphic_data/shader/progress_bar.frag.wgsl"),
        )?;
        let template = material::try_template(mesh_template(
            self.context,
            "health",
            vertex,
            fragment,
            context.surface_format(),
            BlendMode::Alpha,
            vec![
                uniform_group(0, 64, wgpu::ShaderStages::VERTEX),
                uniform_group(2, 4, wgpu::ShaderStages::FRAGMENT),
            ],
        ))?
        .share();
        Ok(self.nodus_materials.insert(template))
    }
    pub fn load_texture(
        &mut self,
        context: &super::GpuContext,
        path: impl AsRef<Path>,
    ) -> Result<TextureId, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(owner) = self.textures.get(path) {
            return Ok(owner.id());
        }
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let raw = context.device.create_texture(&wgpu::TextureDescriptor {
            label: path.to_str(),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        context.queue.write_texture(
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
        let owner =
            texture::try_register_in(self.context, texture::Texture::from_raw(raw), &self.sampler)?
                .share();
        let id = owner.id();
        self.textures.insert(path.to_path_buf(), owner);
        Ok(id)
    }
    pub fn texture_size(&self, id: TextureId) -> RenderResult<(u32, u32)> {
        let texture =
            texture::get(id).ok_or_else(|| RenderError::new("texture", "missing texture"))?;
        Ok((texture.as_raw().width(), texture.as_raw().height()))
    }
    pub(crate) fn texture_binding(
        &mut self,
        texture: TextureId,
    ) -> RenderResult<SharedResource<gpu::BindingSetId>> {
        let value =
            texture::get(texture).ok_or_else(|| RenderError::new("texture", "missing texture"))?;
        if let Some(owner) = self.texture_bindings.get(&texture) {
            return Ok(owner.clone());
        }
        let owner = bindings::try_register(BindingSet {
            context: self.context,
            group: 1,
            entries: vec![
                (0, BindingResource::Texture(value.view().clone())),
                (1, BindingResource::Sampler(self.sampler.as_raw().clone())),
            ],
        })?
        .share();
        self.texture_bindings.insert(texture, owner.clone());
        Ok(owner)
    }
    pub(crate) fn sprite_material(
        &mut self,
        texture: TextureId,
        blend: BlendMode,
        flash: bool,
    ) -> RenderResult<MaterialId> {
        let binding = self.texture_binding(texture)?;
        if let Some(owner) = self.sprite_materials.get(&(texture, blend, flash)) {
            return Ok(owner.id());
        }
        let owner = material::try_register(material::Material {
            template: self.sprite_templates[&(blend, flash)].clone(),
            bindings: vec![binding],
        })?
        .share();
        let id = owner.id();
        self.sprite_materials.insert((texture, blend, flash), owner);
        Ok(id)
    }
    pub fn register_nodus_texture(
        &mut self,
        id: TextureId,
    ) -> RenderResult<prism_legacy::ids::TextureId> {
        self.texture_size(id)?;
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
        self.texture_size(texture)?;
        Ok(texture)
    }
    pub fn unregister_nodus_texture(&mut self, id: prism_legacy::ids::TextureId) -> bool {
        self.nodus_textures.remove(id).is_some()
    }
    pub fn remove_texture(&mut self, id: TextureId) -> bool {
        let owned = self.textures.values().any(|owner| owner.id() == id);
        self.sprite_materials
            .retain(|(texture, _, _), _| *texture != id);
        self.texture_bindings.remove(&id);
        self.textures.retain(|_, owner| owner.id() != id);
        owned
    }
}

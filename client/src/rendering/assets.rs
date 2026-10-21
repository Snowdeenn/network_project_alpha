//! Persistent assets and binding schemas for the Prism V2 backend.
use super::BlendMode;
use prism::{bindings::*, ids::*, material::*, pipeline::VertexLayout, resource::ResourceManager};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct RenderAssets {
    pub gpu: ResourceManager,
    textures: HashMap<PathBuf, TextureId>,
    texture_bindings: HashMap<TextureId, BindingSetHandle>,
    sprite_materials: HashMap<(TextureId, BlendMode, bool), MaterialHandle>,
    pub(crate) shapes: HashMap<BlendMode, MaterialHandle>,
    sprite_templates: HashMap<(BlendMode, bool), MaterialTemplateHandle>,
    sampler: SamplerHandle,
    pub(crate) nodus_materials:
        prism_legacy::arena::Arena<MaterialTemplateHandle, prism_legacy::ids::MaterialTag>,
    nodus_textures: prism_legacy::arena::Arena<TextureId, prism_legacy::ids::TextureTag>,
}

pub(crate) fn uniform_group(index: u32, size: u64, visibility: wgpu::ShaderStages) -> GroupSchema {
    GroupSchema {
        index,
        entries: vec![BindingEntryDesc {
            binding: 0,
            visibility,
            kind: BindingKind::Uniform {
                min_size: size,
                dynamic_offset: false,
            },
            count: None,
        }],
    }
}

pub(crate) fn texture_group() -> GroupSchema {
    GroupSchema {
        index: 1,
        entries: vec![
            BindingEntryDesc {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                kind: BindingKind::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindingEntryDesc {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                kind: BindingKind::Sampler {
                    kind: wgpu::SamplerBindingType::Filtering,
                },
                count: None,
            },
        ],
    }
}

pub(crate) fn mesh_template(
    label: &str,
    vertex: ShaderId,
    fragment: ShaderId,
    format: wgpu::TextureFormat,
    blend: BlendMode,
    groups: Vec<GroupSchema>,
) -> MaterialTemplate {
    let mut template = MaterialTemplate::fullscreen(label, vertex, format);
    template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
    template.schema = BindingSchema { groups };
    template.vertex_layouts = vec![Some(VertexLayout {
        stride: std::mem::size_of::<prism::Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4]
            .to_vec(),
    })];
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
    pub fn create_health_material(
        &mut self,
        context: &prism::GpuContext,
    ) -> prism::RenderResult<prism_legacy::ids::MaterialId> {
        let vertex = self.gpu.create_shader(
            "HUD vertex",
            include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
        )?;
        let fragment = self.gpu.create_shader(
            "health",
            include_str!("../graphic_data/shader/progress_bar.frag.wgsl"),
        )?;
        let template = mesh_template(
            "health",
            vertex,
            fragment,
            context.surface_format(),
            BlendMode::Alpha,
            vec![
                uniform_group(0, 64, wgpu::ShaderStages::VERTEX),
                uniform_group(2, 4, wgpu::ShaderStages::FRAGMENT),
            ],
        );
        let template = self.gpu.create_template(template)?;
        Ok(self.nodus_materials.insert(template))
    }

    pub fn new(context: &prism::GpuContext) -> prism::RenderResult<Self> {
        let mut gpu = ResourceManager::from_context(context);
        let vertex = gpu.create_shader(
            "scene vertex",
            include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
        )?;
        let flat = gpu.create_shader(
            "scene color",
            include_str!("../graphic_data/shader/default.frag.wgsl"),
        )?;
        let textured = gpu.create_shader(
            "scene texture",
            include_str!("../graphic_data/shader/default_textured.frag.wgsl"),
        )?;
        let mut shapes = HashMap::new();
        let flash_shader = gpu.create_shader(
            "entity tint",
            include_str!("../graphic_data/shader/entity_flash.frag.wgsl"),
        )?;
        let mut sprite_templates = HashMap::new();
        for blend in [BlendMode::Alpha, BlendMode::Additive] {
            let camera = uniform_group(0, 64, wgpu::ShaderStages::VERTEX);
            let template = gpu.create_template(mesh_template(
                "shape",
                vertex,
                flat,
                context.surface_format(),
                blend,
                vec![camera.clone()],
            ))?;
            shapes.insert(
                blend,
                gpu.create_material(Material {
                    template,
                    bindings: vec![],
                    parameters: None,
                })?,
            );
            let template = gpu.create_template(mesh_template(
                "sprite",
                vertex,
                textured,
                context.surface_format(),
                blend,
                vec![camera, texture_group()],
            ))?;
            sprite_templates.insert((blend, false), template);
            let template = gpu.create_template(mesh_template(
                "entity tint",
                vertex,
                flash_shader,
                context.surface_format(),
                blend,
                vec![
                    uniform_group(0, 64, wgpu::ShaderStages::VERTEX),
                    texture_group(),
                ],
            ))?;
            sprite_templates.insert((blend, true), template);
        }
        let sampler = gpu.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel art sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })?;
        Ok(Self {
            gpu,
            textures: HashMap::new(),
            texture_bindings: HashMap::new(),
            sprite_materials: HashMap::new(),
            shapes,
            sprite_templates,
            sampler,
            nodus_materials: prism_legacy::arena::Arena::new(),
            nodus_textures: prism_legacy::arena::Arena::new(),
        })
    }

    pub fn load_texture(
        &mut self,
        context: &prism::GpuContext,
        path: impl AsRef<Path>,
    ) -> Result<TextureId, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(id) = self.textures.get(path) {
            return Ok(*id);
        }
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let mut desc =
            prism::graph::TextureDesc::color(width, height, wgpu::TextureFormat::Rgba8UnormSrgb);
        desc.usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let id = self
            .gpu
            .create_texture(&path.display().to_string(), &desc)?;
        context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: self.gpu.texture(id)?.raw(),
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
            desc.size,
        );
        self.textures.insert(path.to_path_buf(), id);
        Ok(id)
    }

    pub fn texture_size(&self, id: TextureId) -> prism::RenderResult<(u32, u32)> {
        let texture = self.gpu.texture(id)?.raw();
        Ok((texture.width(), texture.height()))
    }

    pub(crate) fn sprite_material(
        &mut self,
        texture: TextureId,
        blend: BlendMode,
        flash: bool,
    ) -> prism::RenderResult<MaterialHandle> {
        if let Some(id) = self.sprite_materials.get(&(texture, blend, flash)) {
            return Ok(*id);
        }
        let bindings = if let Some(id) = self.texture_bindings.get(&texture) {
            *id
        } else {
            let view = self.gpu.create_view(texture, &Default::default())?;
            let id = self.gpu.create_bindings(BindingSet {
                group: 1,
                entries: vec![
                    (0, BindingResource::Texture(view)),
                    (1, BindingResource::Sampler(self.sampler)),
                ],
            });
            self.texture_bindings.insert(texture, id);
            id
        };
        let material = self.gpu.create_material(Material {
            template: self.sprite_templates[&(blend, flash)],
            bindings: vec![bindings],
            parameters: None,
        })?;
        self.sprite_materials
            .insert((texture, blend, flash), material);
        Ok(material)
    }

    /// Give Nodus an opaque UI token; it never owns a V2 GPU resource.
    pub fn register_nodus_texture(
        &mut self,
        id: TextureId,
    ) -> prism::RenderResult<prism_legacy::ids::TextureId> {
        self.gpu.texture(id)?;
        Ok(self.nodus_textures.insert(id))
    }

    pub(crate) fn nodus_texture(
        &self,
        id: prism_legacy::ids::TextureId,
    ) -> prism::RenderResult<TextureId> {
        self.nodus_textures
            .get(id)
            .copied()
            .ok_or_else(|| prism::RenderError::new("Nodus texture", "unregistered UI token"))
    }
}

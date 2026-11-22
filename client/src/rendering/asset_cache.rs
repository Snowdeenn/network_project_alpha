//! Non-owning asset lookups. GPU owners live in the application RenderData.
use super::resource_loader::mesh_template;
use super::shader_bindings::{self, SHAPE_BINDINGS, SPRITE_BINDINGS, TextureSlots};
use super::{BlendMode, TextureId};
use gpu::{RenderError, RenderResult};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use velum::{
    RenderData, RenderDataId, SharedResource,
    backend::{
        self as gpu,
        bindings::BindingResource,
        context::ContextId,
        material::{self, MaterialId, MaterialTemplateId},
        sampler, texture,
    },
};

pub struct AssetCache {
    pub context: Option<ContextId>,
    textures: HashMap<PathBuf, (TextureId, RenderDataId<SharedResource<TextureId>>)>,
    texture_bindings:
        HashMap<(TextureId, TextureSlots), Vec<RenderDataId<SharedResource<gpu::BindingSetId>>>>,
    sprite_materials:
        HashMap<(TextureId, BlendMode, bool), RenderDataId<SharedResource<MaterialId>>>,
    pub(crate) shapes: HashMap<BlendMode, RenderDataId<SharedResource<MaterialId>>>,
    sprite_templates: HashMap<(BlendMode, bool), RenderDataId<SharedResource<MaterialTemplateId>>>,
    pub(crate) sampler: RenderDataId<sampler::Sampler<sampler::Filtering>>,
}

impl AssetCache {
    pub fn new(context: &super::GpuContext, data: &mut RenderData) -> RenderResult<Self> {
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
            let template = material::try_template(mesh_template(
                ctx,
                "shape",
                vertex.clone(),
                flat.clone(),
                context.surface_format(),
                blend,
                SHAPE_BINDINGS.schema()?,
            ))?
            .share();
            shapes.insert(
                blend,
                data.insert(
                    material::try_register(material::Material {
                        template,
                        bindings: vec![],
                    })?
                    .share(),
                ),
            );
            for (is_flash, fragment) in [(false, &textured), (true, &flash)] {
                let template = material::try_template(mesh_template(
                    ctx,
                    "sprite",
                    vertex.clone(),
                    fragment.clone(),
                    context.surface_format(),
                    blend,
                    SPRITE_BINDINGS.schema()?,
                ))?
                .share();
                sprite_templates.insert((blend, is_flash), data.insert(template));
            }
        }
        Ok(Self {
            context: ctx,
            textures: HashMap::new(),
            texture_bindings: HashMap::new(),
            sprite_materials: HashMap::new(),
            shapes,
            sprite_templates,
            sampler: data.insert(sampler::pixel_art("pixel art sampler")),
        })
    }
    pub fn load_texture(
        &mut self,
        context: &super::GpuContext,
        data: &mut RenderData,
        path: impl AsRef<Path>,
    ) -> Result<TextureId, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(&(id, owner)) = self.textures.get(path) {
            if data.get(owner).is_some() {
                return Ok(id);
            }
        }
        let raw = super::resource_loader::load_texture(context, path)?;
        let (owner, _) = texture::register_texture(raw);
        let owner = owner.share();
        let id = owner.id();
        self.textures
            .insert(path.to_path_buf(), (id, data.insert(owner)));
        Ok(id)
    }
    pub fn texture_size(&self, id: TextureId) -> RenderResult<(u32, u32)> {
        super::resource_loader::texture_size(id)
    }
    pub(crate) fn texture_binding(
        &mut self,
        texture: TextureId,
        data: &mut RenderData,
        slots: TextureSlots,
    ) -> RenderResult<Vec<SharedResource<gpu::BindingSetId>>> {
        let storage = gpu::context::texture_storage(None);
        let storage = storage.read().unwrap();
        let resource = storage
            .get(texture)
            .ok_or_else(|| RenderError::new("texture", "missing texture"))?;
        let value = match resource {
            texture::TextureResource::Separate(resource)
            | texture::TextureResource::Array(resource)
            | texture::TextureResource::Atlas { resource, .. } => &resource.texture,
        };
        if let Some(owners) = self.texture_bindings.get(&(texture, slots)) {
            return owners
                .iter()
                .map(|id| {
                    data.require(*id)
                        .cloned()
                        .map_err(|e| RenderError::new("asset cache", e.to_string()))
                })
                .collect();
        }
        let owners = shader_bindings::register_sets(
            self.context,
            [
                (
                    slots.texture,
                    BindingResource::Texture(value.view().clone()),
                ),
                (
                    slots.sampler,
                    BindingResource::Sampler(
                        data.require(self.sampler)
                            .map_err(|e| RenderError::new("sampler", e.to_string()))?
                            .as_raw()
                            .clone(),
                    ),
                ),
            ],
        )?;
        self.texture_bindings.insert(
            (texture, slots),
            owners
                .iter()
                .map(|owner| data.insert(owner.clone()))
                .collect(),
        );
        Ok(owners)
    }
    pub(crate) fn sprite_material(
        &mut self,
        texture: TextureId,
        data: &mut RenderData,
        blend: BlendMode,
        flash: bool,
    ) -> RenderResult<MaterialId> {
        let bindings = self.texture_binding(texture, data, SPRITE_BINDINGS.texture.unwrap())?;
        if let Some(owner) = self.sprite_materials.get(&(texture, blend, flash)) {
            return data
                .require(*owner)
                .map(|owner| owner.id())
                .map_err(|e| RenderError::new("asset cache", e.to_string()));
        }
        let owner = material::try_register(material::Material {
            template: data
                .require(self.sprite_templates[&(blend, flash)])
                .map_err(|e| RenderError::new("sprite template", e.to_string()))?
                .clone(),
            bindings,
        })?
        .share();
        let id = owner.id();
        self.sprite_materials
            .insert((texture, blend, flash), data.insert(owner));
        Ok(id)
    }
    pub fn remove_texture(&mut self, data: &mut RenderData, id: TextureId) -> bool {
        self.sprite_materials.retain(|(texture, _, _), owner| {
            if *texture == id {
                data.remove(*owner);
                false
            } else {
                true
            }
        });
        self.texture_bindings.retain(|(texture, _), owners| {
            if *texture == id {
                for owner in owners {
                    data.remove(*owner);
                }
                false
            } else {
                true
            }
        });
        let mut removed = false;
        self.textures.retain(|_, (texture, owner)| {
            if *texture == id {
                removed |= data.remove(*owner).is_some();
                false
            } else {
                true
            }
        });
        removed
    }
}

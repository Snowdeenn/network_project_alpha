//! Non-owning asset lookups. GPU owners live in the application RenderData.
use super::material::mesh_template;
use super::shader_bindings::{self, SPRITE_BINDINGS, TextureSlots};
use super::{BlendMode, TextureId};
use gpu::{RenderError, RenderResult};
use std::{collections::HashMap, path::PathBuf};
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SpriteProgram {
    Standard,
    NinePatch,
    Flash,
}

pub(super) const SPRITE_SLOTS: gpu::standard::Slots = gpu::standard::Slots {
    instances: (2, 0),
    camera: (0, 0),
    texture: (1, 0),
    sampler: (1, 1),
    vertices: 0,
};

pub struct AssetCache {
    pub context: Option<ContextId>,
    pub(super) textures: HashMap<PathBuf, (TextureId, RenderDataId<SharedResource<TextureId>>)>,
    texture_bindings:
        HashMap<(TextureId, TextureSlots), Vec<RenderDataId<SharedResource<gpu::BindingSetId>>>>,
    sprite_materials:
        HashMap<(TextureId, BlendMode, SpriteProgram), RenderDataId<SharedResource<MaterialId>>>,
    pub(crate) shapes: HashMap<BlendMode, RenderDataId<SharedResource<MaterialId>>>,
    sprite_templates:
        HashMap<(BlendMode, SpriteProgram), RenderDataId<SharedResource<MaterialTemplateId>>>,
    pub(crate) sampler: RenderDataId<sampler::Sampler<sampler::Filtering>>,
}

impl AssetCache {
    pub fn new(context: &super::GpuContext, data: &mut RenderData) -> RenderResult<Self> {
        let ctx = context.context_id();
        let vertex = gpu::try_shader(
            ctx,
            "scene vertex",
            include_str!("../graphic_data/shader/default_textured.vert.wgsl"),
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
            let template = gpu::standard::try_template(
                ctx,
                context.surface_format(),
                SPRITE_SLOTS,
                None,
                Some(super::material::blend_state(blend)),
                None,
            )?
            .share();
            shapes.insert(
                blend,
                data.insert(
                    velum::try_material!(context_id: ctx, template: template, bindings: [])?
                        .share(),
                ),
            );
            let standard = gpu::standard::try_template(
                ctx,
                context.surface_format(),
                SPRITE_SLOTS,
                Some(false),
                Some(super::material::blend_state(blend)),
                None,
            )?;
            sprite_templates.insert(
                (blend, SpriteProgram::Standard),
                data.insert(standard.share()),
            );
            for (program, fragment) in [
                (SpriteProgram::NinePatch, &textured),
                (SpriteProgram::Flash, &flash),
            ] {
                let template = mesh_template(
                    ctx,
                    "sprite",
                    vertex.clone(),
                    fragment.clone(),
                    context.surface_format(),
                    blend,
                    SPRITE_BINDINGS.schema()?,
                )?;
                sprite_templates.insert((blend, program), data.insert(template));
            }
        }
        Ok(Self {
            context: ctx,
            textures: HashMap::new(),
            texture_bindings: HashMap::new(),
            sprite_materials: HashMap::new(),
            shapes,
            sprite_templates,
            sampler: data.insert(sampler::pixel_art(ctx, "pixel art sampler")),
        })
    }
    pub(crate) fn texture_binding(
        &mut self,
        texture: TextureId,
        data: &mut RenderData,
        slots: TextureSlots,
    ) -> RenderResult<Vec<SharedResource<gpu::BindingSetId>>> {
        if texture.context_id() != self.context {
            return Err(RenderError::new(
                "texture",
                "texture belongs to another GPU context",
            ));
        }
        let resource =
            texture::get(texture).ok_or_else(|| RenderError::new("texture", "missing texture"))?;
        let value = match resource.as_ref() {
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
    pub(super) fn sprite_material(
        &mut self,
        texture: TextureId,
        data: &mut RenderData,
        blend: BlendMode,
        program: SpriteProgram,
    ) -> RenderResult<MaterialId> {
        let bindings = self.texture_binding(texture, data, SPRITE_BINDINGS.texture.unwrap())?;
        if let Some(owner) = self.sprite_materials.get(&(texture, blend, program)) {
            return data
                .require(*owner)
                .map(|owner| owner.id())
                .map_err(|e| RenderError::new("asset cache", e.to_string()));
        }
        let template = data
            .require(self.sprite_templates[&(blend, program)])
            .map_err(|e| RenderError::new("sprite template", e.to_string()))?
            .clone();
        let owner =
            material::try_register_in(material::Material { template, bindings }, self.context)?
                .share();
        let id = owner.id();
        self.sprite_materials
            .insert((texture, blend, program), data.insert(owner));
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

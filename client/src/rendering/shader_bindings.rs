//! Shader contracts, independent of Prism's historical group numbering.
//! Locations belong to each shader program, never to the renderer globally.
use std::collections::BTreeMap;
use velum::backend::{
    BindingSchema, GroupSchema, RenderError, RenderResult,
    bindings::{self, BindingResource, BindingSet},
    context::ContextId,
};
use velum::{OwnedResource, SharedResource};

/// Native per-object contributions, including storage, arrays and dynamic offsets.
#[derive(Clone, Default)]
pub struct ObjectBindings(pub Vec<bindings::BindingUse>);
impl std::fmt::Debug for ObjectBindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.0
                    .iter()
                    .map(|usage| (usage.set, &usage.dynamic_offsets, usage.replace)),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingLocation {
    pub group: u32,
    pub binding: u32,
}
impl BindingLocation {
    pub const fn new(group: u32, binding: u32) -> Self {
        Self { group, binding }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniformSlot {
    pub location: BindingLocation,
    pub size: u64,
    pub visibility: wgpu::ShaderStages,
}
impl UniformSlot {
    fn declaration(self) -> RenderResult<(BindingLocation, wgpu::BindGroupLayoutEntry)> {
        if self.size == 0 || self.size % 4 != 0 {
            return Err(RenderError::new(
                "uniform contract",
                "size must be nonzero and a multiple of four",
            ));
        }
        Ok((self.location, self.entry()))
    }
    pub fn entry(self) -> wgpu::BindGroupLayoutEntry {
        bindings::buffer_entry(
            self.location.binding,
            self.visibility,
            wgpu::BufferBindingType::Uniform,
            self.size,
            false,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureSlots {
    pub texture: BindingLocation,
    pub sampler: BindingLocation,
}
impl TextureSlots {
    fn entries(self) -> [(BindingLocation, wgpu::BindGroupLayoutEntry); 2] {
        [
            (
                self.texture,
                wgpu::BindGroupLayoutEntry {
                    binding: self.texture.binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ),
            (
                self.sampler,
                wgpu::BindGroupLayoutEntry {
                    binding: self.sampler.binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshBindings {
    pub camera: UniformSlot,
    pub texture: Option<TextureSlots>,
    pub parameters: Option<UniformSlot>,
}
impl MeshBindings {
    pub fn schema(self) -> RenderResult<BindingSchema> {
        let mut entries = vec![self.camera.declaration()?];
        if let Some(texture) = self.texture {
            entries.extend(texture.entries());
        }
        if let Some(parameters) = self.parameters {
            entries.push(parameters.declaration()?);
        }
        schema(entries)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EffectBindings {
    pub source: TextureSlots,
    pub parameters: Option<UniformSlot>,
}
impl EffectBindings {
    /// Contract for Alpha's bundled fullscreen shaders; custom programs can supply their own.
    pub fn bundled(parameter_size: u64) -> Self {
        Self {
            source: TextureSlots {
                texture: BindingLocation::new(0, 0),
                sampler: BindingLocation::new(0, 1),
            },
            parameters: (parameter_size != 0).then_some(UniformSlot {
                location: BindingLocation::new(1, 0),
                size: parameter_size,
                visibility: wgpu::ShaderStages::FRAGMENT,
            }),
        }
    }
    pub fn schema(self) -> RenderResult<BindingSchema> {
        let mut entries = self.source.entries().to_vec();
        if let Some(parameters) = self.parameters {
            entries.push(parameters.declaration()?);
        }
        schema(entries)
    }
}

// These declarations describe the bundled WGSL files, not reserved engine slots.
pub const SHAPE_BINDINGS: MeshBindings = MeshBindings {
    camera: UniformSlot {
        location: BindingLocation::new(0, 0),
        size: 64,
        visibility: wgpu::ShaderStages::VERTEX,
    },
    texture: None,
    parameters: None,
};
pub const SPRITE_BINDINGS: MeshBindings = MeshBindings {
    texture: Some(TextureSlots {
        texture: BindingLocation::new(1, 0),
        sampler: BindingLocation::new(1, 1),
    }),
    ..SHAPE_BINDINGS
};
pub const HEALTH_BINDINGS: MeshBindings = MeshBindings {
    parameters: Some(UniformSlot {
        location: BindingLocation::new(2, 0),
        size: 4,
        visibility: wgpu::ShaderStages::FRAGMENT,
    }),
    ..SHAPE_BINDINGS
};

/// Merge declarations by group, rejecting overlapping contributions instead of overwriting.
pub fn schema(
    entries: impl IntoIterator<Item = (BindingLocation, wgpu::BindGroupLayoutEntry)>,
) -> RenderResult<BindingSchema> {
    let mut groups = BTreeMap::<u32, BTreeMap<u32, wgpu::BindGroupLayoutEntry>>::new();
    for (location, entry) in entries {
        if entry.binding != location.binding || entry.visibility.is_empty() {
            return Err(RenderError::new(
                "binding schema",
                "inconsistent location or empty visibility",
            ));
        }
        if groups
            .entry(location.group)
            .or_default()
            .insert(location.binding, entry)
            .is_some()
        {
            return Err(RenderError::new(
                format!("group {} binding {}", location.group, location.binding),
                "conflicting shader declarations",
            ));
        }
    }
    BindingSchema {
        groups: groups
            .into_iter()
            .map(|(index, entries)| GroupSchema {
                index,
                entries: entries.into_values().collect(),
            })
            .collect(),
    }
    .normalized()
}

/// Register explicit resources. Uniform/storage buffers and arrays are native Velum resources.
/// Dynamic offsets are supplied separately with `BindingUse::with_offsets` at draw time.
pub fn register_sets(
    context: Option<ContextId>,
    contributions: impl IntoIterator<Item = (BindingLocation, BindingResource)>,
) -> RenderResult<Vec<SharedResource<bindings::BindingSetId>>> {
    let mut groups = BTreeMap::<u32, BTreeMap<u32, BindingResource>>::new();
    for (location, resource) in contributions {
        if groups
            .entry(location.group)
            .or_default()
            .insert(location.binding, resource)
            .is_some()
        {
            return Err(RenderError::new(
                format!("group {} binding {}", location.group, location.binding),
                "conflicting resource contributions",
            ));
        }
    }
    groups
        .into_iter()
        .map(|(group, entries)| {
            bindings::try_register(BindingSet {
                context,
                group,
                entries: entries.into_iter().collect(),
            })
            .map(OwnedResource::share)
        })
        .collect()
}

#[cfg(test)]
mod tests;

pub(super) fn uniform_set(
    context: Option<velum::backend::context::ContextId>,
    slot: UniformSlot,
    bytes: &[u8],
) -> RenderResult<SharedResource<velum::backend::BindingSetId>> {
    use velum::{
        OwnedResource,
        backend::{buffer, geometry::BufferRegion},
    };
    if bytes.len() as u64 != slot.size || bytes.is_empty() || bytes.len() % 4 != 0 {
        return Err(RenderError::new(
            "uniform",
            "data size differs from shader contract",
        ));
    }
    let owner = buffer::allocate_in::<u8, buffer::Uniform>(context, slot.size)
        .map_err(|e| RenderError::new("uniform", e.to_string()))?
        .share();
    buffer::try_write_buffer(&owner.id(), 0, bytes)
        .map_err(|e| RenderError::new("uniform upload", e.to_string()))?;
    velum::try_binding_set! {
        context: context,
        group: slot.location.group,
        entries: [slot.location.binding => BindingResource::buffer(BufferRegion::from_shared(owner)?)],
    }
    .map(OwnedResource::share)
}

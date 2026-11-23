//! Convert Alpha drawing commands to ordered Velum objects.
use super::{
    AssetCache, Primitive, SceneFrame,
    asset_cache::{SPRITE_SLOTS, SpriteProgram},
    renderer::render_data_error,
    shader_bindings::{self, SHAPE_BINDINGS, SPRITE_BINDINGS, UniformSlot, uniform_set},
};
use std::ops::Range;
use velum::{
    OwnedResource, RenderData,
    backend::{
        self as gpu, RenderError, RenderResult,
        bindings::BindingUse,
        geometry::{self, Geometry},
        material::{self, Material, MaterialId},
        object::{PhaseId, RenderObject, RenderPhase},
    },
    geometry::{Mesh, Shape, Tesselator},
};
struct Solid {
    geometry: gpu::GeometryId,
    material: MaterialId,
    instance: gpu::sprite::RenderInstance,
}
impl gpu::instances::Instance for Solid {}
impl gpu::InstanceSource for Solid {
    type Gpu = gpu::sprite::RenderInstance;
    fn gpu_data(&self) -> Self::Gpu {
        self.instance
    }
    fn binding() -> gpu::InstanceBinding {
        SPRITE_SLOTS.instance_binding()
    }
}
impl gpu::ObjectInstance for Solid {
    fn geometry(&self) -> gpu::GeometryId {
        self.geometry
    }
    fn material(&self) -> MaterialId {
        self.material
    }
}
struct SceneSprite {
    sprite: gpu::sprite::Sprite,
    gpu: gpu::sprite::RenderInstance,
}
impl gpu::instances::Instance for SceneSprite {}
impl gpu::sprite::SpriteInstance for SceneSprite {
    fn render_instance(&self) -> gpu::sprite::RenderInstance {
        self.gpu
    }
    fn sprite(&self) -> &gpu::sprite::Sprite {
        &self.sprite
    }
    fn sprite_mut(&mut self) -> &mut gpu::sprite::Sprite {
        &mut self.sprite
    }
}

pub(super) struct SceneGeometry {
    quad: OwnedResource<gpu::GeometryId>,
    pub(super) fullscreen: OwnedResource<gpu::GeometryId>,
    cameras: std::collections::HashMap<shader_bindings::BindingLocation, gpu::standard::Cameras>,
    geometries: Vec<OwnedResource<gpu::GeometryId>>,
    pub(super) materials: Vec<OwnedResource<MaterialId>>,
    mesh: Mesh,
}
impl SceneGeometry {
    pub(super) fn new(ctx: Option<gpu::context::ContextId>) -> RenderResult<Self> {
        let mut quad = Mesh::new();
        Tesselator::tesselate(
            &Shape::Quad {
                pos: [-0.5; 2],
                size: [1.; 2],
                rotation: 0.,
                color: [1.; 4],
                uv: None,
            },
            &mut quad,
        );
        Ok(Self {
            quad: geometry::try_mesh(ctx, &quad)?,
            fullscreen: geometry::try_register(Geometry::generated(ctx, 0..3))?,
            cameras: Default::default(),
            geometries: Vec::new(),
            materials: Vec::new(),
            mesh: Mesh::with_capacity(4096, 6144),
        })
    }
    fn tessellate(
        &mut self,
        assets: &mut AssetCache,
        data: &mut RenderData,
        command: &Primitive,
    ) -> RenderResult<Option<(MaterialId, UniformSlot, Vec<BindingUse>)>> {
        let (shape, material) = match command {
            Primitive::Shape { .. } => return Ok(None),
            Primitive::Texture {
                id,
                pos,
                size,
                rotation,
                uv,
                tint,
                blend,
                ..
            } => (
                Shape::Quad {
                    pos: *pos,
                    size: *size,
                    rotation: *rotation,
                    uv: *uv,
                    color: *tint,
                },
                assets.sprite_material(*id, data, *blend, SpriteProgram::Flash)?,
            ),
            Primitive::NinePatch {
                id,
                pos,
                size,
                texture_size,
                margins,
                tint,
                blend,
                ..
            } => (
                Shape::NinePatch {
                    pos: *pos,
                    size: *size,
                    texture_size: *texture_size,
                    margins: *margins,
                    color: *tint,
                },
                assets.sprite_material(*id, data, *blend, SpriteProgram::NinePatch)?,
            ),
            Primitive::Material {
                template,
                layout,
                resource_bindings,
                texture_id,
                pos,
                size,
                rotation,
                uv,
                tint,
                uniform_data,
                ..
            } => {
                let template = template.clone();
                layout.schema()?;
                if layout.camera.size != 64 {
                    return Err(RenderError::new("camera", "expected a mat4 uniform"));
                }
                let mut bindings = vec![];
                match (texture_id, layout.texture) {
                    (Some(texture), Some(slots)) => {
                        bindings.extend(assets.texture_binding(*texture, data, slots)?)
                    }
                    (None, None) => {}
                    _ => {
                        return Err(RenderError::new(
                            "material texture",
                            "resource differs from shader contract",
                        ));
                    }
                }
                match layout.parameters {
                    Some(slot) => bindings.push(uniform_set(assets.context, slot, uniform_data)?),
                    None if uniform_data.is_empty() => {}
                    None => {
                        return Err(RenderError::new(
                            "material parameters",
                            "shader has no parameter slot",
                        ));
                    }
                }
                let owner =
                    material::try_register_in(Material { template, bindings }, assets.context)?;
                let id = owner.id();
                self.materials.push(owner);
                Tesselator::tesselate(
                    &Shape::Quad {
                        pos: *pos,
                        size: *size,
                        rotation: *rotation,
                        uv: *uv,
                        color: *tint,
                    },
                    &mut self.mesh,
                );
                return Ok(Some((id, layout.camera, resource_bindings.0.clone())));
            }
            Primitive::Instances { .. } | Primitive::Text { .. } => return Ok(None),
        };
        Tesselator::tesselate(&shape, &mut self.mesh);
        Ok(Some((material, SPRITE_BINDINGS.camera, vec![])))
    }
    pub(super) fn prepare(
        &mut self,
        assets: &mut AssetCache,
        data: &mut RenderData,
        scene: &mut SceneFrame,
        width: u32,
        height: u32,
    ) -> RenderResult<[Vec<RenderObject>; 3]> {
        self.geometries.clear();
        self.materials.clear();

        self.mesh.clear();
        let mut view = velum::camera::Camera2D::new();
        view.set_pos(
            scene.camera_pos.x - scene.cam_shake_offset.x,
            scene.camera_pos.y - scene.cam_shake_offset.y,
        );
        for cameras in self.cameras.values() {
            cameras.update(&view, [width, height], 1.0)?;
        }
        enum Draw<'a> {
            Mesh(MaterialId, Vec<BindingUse>, Range<u32>),
            Instances(&'a dyn super::InstanceBatch),
            Sprites(MaterialId, Vec<SceneSprite>),
            Solids(Vec<Solid>),
        }
        let mut batches: Vec<(usize, u8, UniformSlot, Draw)> = Vec::new();
        for (phase, commands) in [&mut scene.world, &mut scene.vfx, &mut scene.hud]
            .into_iter()
            .enumerate()
        {
            for command in commands.iter() {
                let layer = command.layer();
                if let Primitive::Shape { shape, blend, .. } = command {
                    let material = data
                        .require(assets.shapes[blend])
                        .map_err(render_data_error)?
                        .id();
                    let (geometry, color, offset, scale, rotation) = match shape {
                        Shape::Quad {
                            pos,
                            size,
                            rotation,
                            color,
                            ..
                        } => (
                            self.quad.id(),
                            *color,
                            [pos[0] + size[0] * 0.5, pos[1] + size[1] * 0.5],
                            *size,
                            *rotation,
                        ),
                        shape => {
                            let mut mesh = Mesh::new();
                            Tesselator::tesselate(shape, &mut mesh);
                            if mesh.indices().is_empty() {
                                continue;
                            }
                            let owner = geometry::try_mesh(assets.context, &mesh)?;
                            let id = owner.id();
                            self.geometries.push(owner);
                            (id, mesh.vertices()[0].color, [0.; 2], [1.; 2], 0.)
                        }
                    };
                    let solid = Solid {
                        geometry,
                        material,
                        instance: gpu::sprite::RenderInstance {
                            color,
                            offset,
                            scale,
                            rotation,
                            depth: 0.,
                            layer: 0,
                            _padding: 0,
                            uv_min: [0.; 2],
                            uv_max: [1.; 2],
                        },
                    };
                    if let Some((p, l, _, Draw::Solids(items))) = batches.last_mut()
                        && *p == phase
                        && *l == layer
                    {
                        items.push(solid);
                    } else {
                        batches.push((
                            phase,
                            layer,
                            SHAPE_BINDINGS.camera,
                            Draw::Solids(vec![solid]),
                        ));
                    }
                    continue;
                }
                if let Primitive::Texture {
                    flash: false,
                    id,
                    pos,
                    size,
                    rotation,
                    uv,
                    tint,
                    blend,
                    ..
                } = command
                {
                    let material =
                        assets.sprite_material(*id, data, *blend, SpriteProgram::Standard)?;
                    let sprite = SceneSprite {
                        sprite: gpu::sprite::Sprite::Separate {
                            texture: *id,
                            indices: 0..6,
                        },
                        gpu: gpu::sprite::RenderInstance {
                            color: *tint,
                            offset: [pos[0] + size[0] * 0.5, pos[1] + size[1] * 0.5],
                            scale: *size,
                            rotation: *rotation,
                            uv_min: uv.map_or([0.; 2], |r| [r.x, r.y]),
                            uv_max: uv.map_or([1.; 2], |r| [r.x + r.w, r.y + r.h]),
                            layer: 0,
                            depth: 0.,
                            _padding: 0,
                        },
                    };
                    if let Some((p, l, _, Draw::Sprites(m, items))) = batches.last_mut()
                        && *p == phase
                        && *l == layer
                        && *m == material
                    {
                        items.push(sprite);
                    } else {
                        batches.push((
                            phase,
                            layer,
                            SPRITE_BINDINGS.camera,
                            Draw::Sprites(material, vec![sprite]),
                        ));
                    }
                    continue;
                }
                if let Primitive::Instances { batch, camera, .. } = command {
                    batches.push((phase, layer, *camera, Draw::Instances(batch.as_ref())));
                    continue;
                }
                let start = self.mesh.indices().len() as u32;
                let Some((material, camera, resources)) = self.tessellate(assets, data, command)?
                else {
                    continue;
                };
                let end = self.mesh.indices().len() as u32;
                if start == end {
                    continue;
                }
                if let Some((p, l, c, Draw::Mesh(m, r, range))) = batches.last_mut()
                    && *p == phase
                    && *l == layer
                    && *m == material
                    && *c == camera
                    && r.is_empty()
                    && resources.is_empty()
                {
                    range.end = end;
                } else {
                    batches.push((
                        phase,
                        layer,
                        camera,
                        Draw::Mesh(material, resources, start..end),
                    ));
                }
            }
        }
        let mut phases: [Vec<RenderObject>; 3] = Default::default();
        let mesh = if self.mesh.indices().is_empty() {
            None
        } else {
            let owner = geometry::try_mesh(assets.context, &self.mesh)?;
            geometry::get(owner.id())
        };
        for (phase, layer, camera, draw) in batches {
            use std::collections::hash_map::Entry;
            let cameras = match self.cameras.entry(camera.location) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let pair = gpu::standard::Cameras::new(
                        assets.context,
                        (camera.location.group, camera.location.binding),
                    )?;
                    pair.update(&view, [width, height], 1.0)?;
                    entry.insert(pair)
                }
            };
            let camera_binding = if phase == 2 {
                cameras.screen_bindings.id()
            } else {
                cameras.world_bindings.id()
            };
            let (material, resources, elements) = match draw {
                instanced @ (Draw::Instances(_) | Draw::Sprites(_, _) | Draw::Solids(_)) => {
                    let mut native =
                        RenderPhase::new(PhaseId(phase as u32), gpu::SortPolicy::PreserveOrder);
                    match instanced {
                        Draw::Solids(items) => {
                            gpu::instances::submit(assets.context, &items, &mut native)?
                        }
                        Draw::Instances(instances) => {
                            instances.submit(assets.context, &mut native)?
                        }
                        Draw::Sprites(material, sprites) => gpu::standard::submit_sprites(
                            &sprites,
                            self.quad.id(),
                            SPRITE_SLOTS.instance_binding(),
                            &mut native,
                            |_| Ok(material),
                        )?,
                        Draw::Mesh(..) => unreachable!(),
                    }
                    for mut object in native.objects().iter().cloned() {
                        object.sort_key.draw_order = layer.into();
                        object.bindings.push(camera_binding.into());
                        phases[phase].push(object);
                    }
                    continue;
                }
                Draw::Mesh(material, resources, elements) => (material, resources, elements),
            };
            let mesh = mesh.as_deref().expect("mesh batch contains indices");
            let geometry = geometry::try_register(Geometry {
                elements,
                ..mesh.clone()
            })?;
            let mut object = RenderObject::new(geometry.id(), material);
            object.phase = PhaseId(phase as u32);
            object.sort_key.draw_order = layer.into();
            object.bindings.push(camera_binding.into());
            object.bindings.extend(resources);
            phases[phase].push(object);
            self.geometries.push(geometry);
        }
        Ok(phases)
    }
}

//! Alpha scene orchestration using only native Velum resource and draw APIs.
use super::post_process::{PostProcessId, PostProcessSubmission};
use super::{OverlayContext, Primitive, RenderAssets, SceneFrame, text::TextOverlay};
use std::ops::Range;
use velum::geometry::{Shape, Tesselator, mesh::Mesh};
use velum::{
    OwnedResource, SharedResource,
    backend::{
        self as gpu, RenderError, RenderResult,
        bindings::{self, BindingResource, BindingSet},
        buffer::{self, Buffer, BufferKind},
        geometry::{self, BufferRegion, Geometry, IndexBuffer, VertexBuffer},
        material::{self, Material, MaterialId, MaterialTemplate, MaterialTemplateId, ShaderEntry},
        object::{PhaseId, RenderObject, RenderPhase},
        target::{self, RenderTargetDescriptor, TargetSize},
    },
};

type Context = super::GpuContext;
struct MeshBuffer<K: BufferKind + Send + Sync + 'static> {
    owner: SharedResource<Buffer<u8, K>>,
    capacity: u64,
}
impl<K: BufferKind + Send + Sync + 'static> MeshBuffer<K> {
    fn new(context: Option<gpu::context::ContextId>, capacity: u64) -> RenderResult<Self> {
        Ok(Self {
            owner: buffer::allocate_in::<u8, K>(context, capacity)
                .map_err(|e| RenderError::new("buffer allocation", e.to_string()))?
                .share(),
            capacity,
        })
    }
    fn upload(
        &mut self,
        context: Option<gpu::context::ContextId>,
        data: &[u8],
    ) -> RenderResult<BufferRegion> {
        if data.len() as u64 > self.capacity {
            *self = Self::new(context, (data.len() as u64).next_power_of_two())?;
        }
        buffer::try_write_buffer(&self.owner.id(), 0, data)
            .map_err(|e| RenderError::new("buffer upload", e.to_string()))?;
        BufferRegion::from_shared(self.owner.clone())?.subregion(0..data.len() as u64)
    }
}
struct EffectProgram {
    template: SharedResource<MaterialTemplateId>,
    parameter_size: usize,
}
pub struct SceneRenderer {
    pending: Option<SceneFrame>,
    vertices: MeshBuffer<buffer::Vertex>,
    indices: MeshBuffer<buffer::Index>,
    cameras: [(
        SharedResource<Buffer<u8, buffer::Uniform>>,
        SharedResource<gpu::BindingSetId>,
    ); 2],
    geometries: Vec<OwnedResource<gpu::GeometryId>>,
    materials: Vec<OwnedResource<MaterialId>>,
    mesh: Mesh,
    text: TextOverlay,
    post_process: super::post_process::PostProcessRegistry,
    effects: Vec<EffectProgram>,
    blit: SharedResource<MaterialTemplateId>,
}
fn uniform(
    context: Option<gpu::context::ContextId>,
    group: u32,
    bytes: &[u8],
) -> RenderResult<SharedResource<gpu::BindingSetId>> {
    let owner = buffer::allocate_in::<u8, buffer::Uniform>(context, bytes.len() as u64)
        .map_err(|e| RenderError::new("uniform", e.to_string()))?
        .share();
    buffer::try_write_buffer(&owner.id(), 0, bytes)
        .map_err(|e| RenderError::new("uniform upload", e.to_string()))?;
    bindings::try_register(BindingSet {
        context,
        group,
        entries: vec![(
            0,
            BindingResource::buffer(BufferRegion::from_shared(owner)?),
        )],
    })
    .map(OwnedResource::share)
}
fn effect_template(
    context: Option<gpu::context::ContextId>,
    format: wgpu::TextureFormat,
    label: &str,
    fragment: &str,
    parameter_size: usize,
) -> RenderResult<SharedResource<MaterialTemplateId>> {
    let vertex = gpu::try_shader(
        context,
        "post vertex",
        include_str!("../graphic_data/shader/default_post_process.vert.wgsl"),
    )?;
    let fragment = gpu::try_shader(context, label, fragment)?;
    let mut template = MaterialTemplate::fullscreen(context, label, vertex, format);
    template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
    template.schema.groups.push(super::assets::texture_group(0));
    if parameter_size > 0 {
        template.schema.groups.push(super::assets::uniform_group(
            1,
            parameter_size as u64,
            wgpu::ShaderStages::FRAGMENT,
        ));
    }
    material::try_template(template).map(OwnedResource::share)
}
impl SceneRenderer {
    pub fn new(context: &Context, assets: &mut RenderAssets) -> RenderResult<Self> {
        fn camera(
            context: Option<gpu::context::ContextId>,
        ) -> RenderResult<(
            SharedResource<Buffer<u8, buffer::Uniform>>,
            SharedResource<gpu::BindingSetId>,
        )> {
            let owner = buffer::allocate_in::<u8, buffer::Uniform>(context, 64)
                .map_err(|e| RenderError::new("camera", e.to_string()))?
                .share();
            let binding = bindings::try_register(BindingSet {
                context,
                group: 0,
                entries: vec![(
                    0,
                    BindingResource::buffer(BufferRegion::from_shared(owner.clone())?),
                )],
            })?
            .share();
            Ok((owner, binding))
        }
        Ok(Self {
            pending: None,
            vertices: MeshBuffer::new(assets.context, 4096)?,
            indices: MeshBuffer::new(assets.context, 4096)?,
            cameras: [camera(assets.context)?, camera(assets.context)?],
            geometries: vec![],
            materials: vec![],
            mesh: Mesh::with_capacity(4096, 6144),
            text: TextOverlay::new(context),
            post_process: Default::default(),
            effects: vec![],
            blit: effect_template(
                assets.context,
                context.surface_format(),
                "presentation",
                include_str!("../graphic_data/shader/default_post_process.frag.wgsl"),
                0,
            )?,
        })
    }
    pub fn queue(&mut self, frame: SceneFrame) {
        self.pending = Some(frame);
    }
    pub fn take_frame(&mut self) -> Option<SceneFrame> {
        self.pending.take()
    }
    pub fn register_post_effect(
        &mut self,
        assets: &mut RenderAssets,
        format: wgpu::TextureFormat,
        label: &str,
        source: &str,
        parameter_size: u64,
    ) -> RenderResult<PostProcessId> {
        if parameter_size % 4 != 0 {
            return Err(RenderError::new(
                "post-process uniform",
                "uniform size must be a multiple of four bytes",
            ));
        }
        let template = effect_template(
            assets.context,
            format,
            label,
            source,
            parameter_size as usize,
        )?;
        let id = PostProcessId(self.effects.len() as u64);
        self.effects.push(EffectProgram {
            template,
            parameter_size: parameter_size as usize,
        });
        Ok(id)
    }
    pub fn register_vfx_post_effect<T: bytemuck::Pod + 'static>(
        &mut self,
        context: &Context,
        assets: &mut RenderAssets,
        material: &str,
        source: &str,
        encode: impl Fn(&super::PostProcessInput<'_>) -> T + 'static,
    ) -> RenderResult<PostProcessId> {
        self.post_process.check_material(material)?;
        let id = self.register_post_effect(
            assets,
            context.surface_format(),
            material,
            source,
            std::mem::size_of::<T>() as u64,
        )?;
        self.post_process.register(material.to_owned(), id, encode);
        Ok(id)
    }
    pub fn add_post_process_pass<T: bytemuck::Pod>(
        &mut self,
        context: &Context,
        assets: &mut RenderAssets,
        source: &str,
        value: Option<T>,
    ) -> RenderResult<PostProcessId> {
        let bytes = value
            .map(|value| bytemuck::bytes_of(&value).to_vec())
            .unwrap_or_default();
        let id = self.register_post_effect(
            assets,
            context.surface_format(),
            "post-process pass",
            source,
            bytes.len() as u64,
        )?;
        self.post_process.add_manual(id, bytes);
        Ok(id)
    }
    pub fn enable_post_process_pass(&mut self, id: PostProcessId) -> RenderResult<()> {
        self.post_process.enable(id, true)
    }
    pub fn disable_post_process_pass(&mut self, id: PostProcessId) -> RenderResult<()> {
        self.post_process.enable(id, false)
    }
    pub fn write_post_process_uniform<T: bytemuck::Pod>(
        &mut self,
        id: PostProcessId,
        value: T,
    ) -> RenderResult<()> {
        self.post_process.write(id, value)
    }

    fn tessellate(
        &mut self,
        assets: &mut RenderAssets,
        command: &Primitive,
    ) -> RenderResult<Option<MaterialId>> {
        let (shape, material) = match command {
            Primitive::Shape { shape, blend, .. } => {
                Tesselator::tesselate(shape, &mut self.mesh);
                return Ok(Some(assets.shapes[blend].id()));
            }
            Primitive::Texture {
                flash,
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
                assets.sprite_material(*id, *blend, *flash)?,
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
                assets.sprite_material(*id, *blend, false)?,
            ),
            Primitive::Material {
                template,
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
                let mut bindings = vec![];
                if let Some(texture) = texture_id {
                    bindings.push(assets.texture_binding(*texture)?);
                }
                if !uniform_data.is_empty() {
                    bindings.push(uniform(assets.context, 2, uniform_data)?);
                }
                let owner = material::try_register(Material { template, bindings })?;
                let id = owner.id();
                self.materials.push(owner);
                (
                    Shape::Quad {
                        pos: *pos,
                        size: *size,
                        rotation: *rotation,
                        uv: *uv,
                        color: *tint,
                    },
                    id,
                )
            }
            Primitive::Text { .. } => return Ok(None),
        };
        Tesselator::tesselate(&shape, &mut self.mesh);
        Ok(Some(material))
    }
    fn prepare(
        &mut self,
        assets: &mut RenderAssets,
        scene: &mut SceneFrame,
        width: u32,
        height: u32,
    ) -> RenderResult<[Vec<RenderObject>; 3]> {
        self.geometries.clear();
        self.materials.clear();
        self.mesh.clear();
        let projection = utils::math::Mat4::orthographic_wgpu(
            0.0,
            width.max(1) as f32,
            height.max(1) as f32,
            0.0,
            -1.0,
            1.0,
        );
        let world = projection.multiply(utils::math::Mat4::translation(
            -scene.camera_pos.x + width as f32 * 0.5 + scene.cam_shake_offset.x,
            -scene.camera_pos.y + height as f32 * 0.5 + scene.cam_shake_offset.y,
            0.0,
        ));
        for (index, matrix) in [world, projection].iter().enumerate() {
            buffer::try_write_buffer(&self.cameras[index].0.id(), 0, bytemuck::bytes_of(matrix))
                .map_err(|e| RenderError::new("camera upload", e.to_string()))?;
        }
        let mut batches: Vec<(usize, MaterialId, Range<u32>)> = vec![];
        for (phase, commands) in [&mut scene.world, &mut scene.vfx, &mut scene.hud]
            .into_iter()
            .enumerate()
        {
            commands.sort_by_key(Primitive::layer);
            for command in commands.iter() {
                let start = self.mesh.indices().len() as u32;
                let Some(material) = self.tessellate(assets, command)? else {
                    continue;
                };
                let end = self.mesh.indices().len() as u32;
                if start == end {
                    continue;
                }
                if let Some((p, m, range)) = batches.last_mut()
                    && *p == phase
                    && *m == material
                {
                    range.end = end;
                } else {
                    batches.push((phase, material, start..end));
                }
            }
        }
        let mut phases: [Vec<RenderObject>; 3] = Default::default();
        if self.mesh.indices().is_empty() {
            return Ok(phases);
        }
        let vertices = self
            .vertices
            .upload(assets.context, self.mesh.vertices_bytes())?;
        let indices = self
            .indices
            .upload(assets.context, self.mesh.indices_bytes())?;
        for (phase, material, elements) in batches {
            let geometry = geometry::try_register(Geometry {
                context: assets.context,
                vertices: vec![VertexBuffer {
                    slot: 0,
                    region: vertices.clone(),
                }],
                indices: Some(IndexBuffer {
                    region: indices.clone(),
                    format: wgpu::IndexFormat::Uint32,
                }),
                elements,
                base_vertex: 0,
            })?;
            let mut object = RenderObject::new(geometry.id(), material);
            object.phase = PhaseId(phase as u32);
            object
                .bindings
                .push(self.cameras[if phase == 2 { 1 } else { 0 }].1.id().into());
            phases[phase].push(object);
            self.geometries.push(geometry);
        }
        Ok(phases)
    }
    fn draw_phase(
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        objects: Vec<RenderObject>,
        phase: u32,
        clear: bool,
    ) -> RenderResult<()> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Alpha Velum phase"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let mut batch = RenderPhase::new(PhaseId(phase), gpu::SortPolicy::PreserveOrder);
        for object in objects {
            batch.push(object);
        }
        batch.execute(&mut pass)
    }
    fn fullscreen(
        encoder: &mut wgpu::CommandEncoder,
        context: Option<gpu::context::ContextId>,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        template: SharedResource<MaterialTemplateId>,
        parameters: &[u8],
    ) -> RenderResult<()> {
        let source = bindings::try_register(BindingSet {
            context,
            group: 0,
            entries: vec![
                (0, BindingResource::Texture(source.clone())),
                (1, BindingResource::Sampler(sampler.clone())),
            ],
        })?
        .share();
        let mut bindings = vec![source];
        if !parameters.is_empty() {
            bindings.push(uniform(context, 1, parameters)?);
        }
        let material = material::try_register(Material { template, bindings })?;
        let geometry = geometry::try_register(Geometry::generated(context, 0..3))?;
        Self::draw_phase(
            encoder,
            destination,
            vec![RenderObject::new(geometry.id(), material.id())],
            0,
            true,
        )
    }
    pub fn render_to_texture(
        &mut self,
        context: &Context,
        assets: &mut RenderAssets,
        mut scene: SceneFrame,
        target: &wgpu::Texture,
    ) -> RenderResult<gpu::report::FrameReport> {
        let mut frame = gpu::renderer::OffscreenFrame::new(
            assets.context,
            target.create_view(&Default::default()),
        )?;
        let [world, vfx, hud] =
            self.prepare(assets, &mut scene, target.width(), target.height())?;
        let descriptor = RenderTargetDescriptor {
            label: "Alpha scene",
            size: TargetSize::Fixed {
                width: target.width(),
                height: target.height(),
            },
            format: target.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            sample_count: 1,
        };
        let mut intermediate = vec![target::register_in(assets.context, descriptor)];
        let mut source = target::get(intermediate[0].id())
            .ok_or_else(|| RenderError::new("target", "missing scene target"))?;
        let (encoder, output) = frame.render_parts();
        Self::draw_phase(encoder, source.texture().view(), world, 0, true)?;
        Self::draw_phase(encoder, source.texture().view(), vfx, 1, false)?;
        let mut submissions = self
            .post_process
            .submissions(&scene.post_effects, [target.width(), target.height()]);
        submissions.append(&mut scene.post_process);
        for PostProcessSubmission { effect, parameters } in submissions {
            let program = self
                .effects
                .get(effect.0 as usize)
                .ok_or_else(|| RenderError::new("post-process", "unknown effect"))?;
            if parameters.len() != program.parameter_size {
                return Err(RenderError::new(
                    "post-process",
                    "uniform size differs from registered layout",
                ));
            }
            let owner = target::register_in(assets.context, descriptor);
            let destination = target::get(owner.id())
                .ok_or_else(|| RenderError::new("target", "missing effect target"))?;
            Self::fullscreen(
                encoder,
                assets.context,
                source.texture().view(),
                destination.texture().view(),
                assets.sampler.as_raw(),
                program.template.clone(),
                &parameters,
            )?;
            intermediate.push(owner);
            source = destination;
        }
        Self::fullscreen(
            encoder,
            assets.context,
            source.texture().view(),
            output,
            assets.sampler.as_raw(),
            self.blit.clone(),
            &[],
        )?;
        Self::draw_phase(encoder, output, hud, 2, false)?;
        self.text.render(
            context,
            encoder,
            output,
            [target.width(), target.height()],
            &scene.hud,
        )?;
        Ok(frame.finish())
    }
    pub fn render_to_target(
        &mut self,
        context: &Context,
        assets: &mut RenderAssets,
        scene: SceneFrame,
        id: super::RenderTargetId,
    ) -> RenderResult<gpu::report::FrameReport> {
        let target = target::get(id).ok_or_else(|| RenderError::new("target", "missing target"))?;
        self.render_to_texture(context, assets, scene, target.texture().as_raw())
    }
    pub fn render(
        &mut self,
        context: &Context,
        assets: &mut RenderAssets,
        scene: SceneFrame,
    ) -> RenderResult<Option<OverlayContext>> {
        if !context.is_ready() {
            return Ok(None);
        }
        let surface = match context.current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface)
            | wgpu::CurrentSurfaceTexture::Suboptimal(surface) => surface,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                context.reconfigure();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::new("surface", "validation error"));
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
        };
        self.render_to_texture(context, assets, scene, &surface.texture)?;
        Ok(Some(OverlayContext {
            surface_view: surface.texture.create_view(&Default::default()),
            surface_texture: surface,
            encoder: context.create_encoder("surface overlays"),
            size: (context.size.width, context.size.height),
        }))
    }
}

use super::{OverlayContext, Primitive, RenderAssets, SceneFrame, text::TextOverlay};
use prism::{
    FrameV2, GraphRenderer, RawMesh, Shape, Tesselator,
    frame::v2::{PhaseSubmission, UploadRequest, ViewSubmission},
    ids::*,
    material::{BindingResource, BindingSet, Material, ParameterBlock},
    render_object::{RenderObject, RenderPhaseId, SortKey, SortPolicy},
    resource::manager::{Geometry, IndexBuffer, VertexBuffer},
};
use std::ops::Range;

const WORLD: RenderPhaseId = RenderPhaseId(10);
const VFX: RenderPhaseId = RenderPhaseId(11);

struct MeshBuffer {
    id: BufferId,
    capacity: u64,
}

impl MeshBuffer {
    fn new(assets: &mut RenderAssets, usage: wgpu::BufferUsages) -> prism::RenderResult<Self> {
        Ok(Self {
            id: assets.gpu.create_buffer(
                "scene stream",
                4096,
                usage | wgpu::BufferUsages::COPY_DST,
            )?,
            capacity: 4096,
        })
    }

    fn reserve(
        &mut self,
        assets: &mut RenderAssets,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> prism::RenderResult<()> {
        if size > self.capacity {
            let capacity = size.next_power_of_two();
            let id = assets.gpu.create_buffer(
                "scene stream",
                capacity,
                usage | wgpu::BufferUsages::COPY_DST,
            )?;
            assets.gpu.remove_buffer(self.id);
            *self = Self { id, capacity };
        }
        Ok(())
    }
}

pub struct SceneRenderer {
    pub graph: GraphRenderer,
    pending: Option<SceneFrame>,
    vertices: MeshBuffer,
    indices: MeshBuffer,
    cameras: [(BufferId, BindingSetHandle); 2],
    geometries: Vec<GeometryHandle>,
    materials: Vec<MaterialHandle>,
    mesh: RawMesh,
    text: TextOverlay,
    post_process: super::post_process::PostProcessRegistry,
}

impl SceneRenderer {
    /// Register a material once; every active VFX occurrence supplies its own uniform values.
    pub fn register_vfx_post_effect<T: bytemuck::Pod + 'static>(
        &mut self,
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
        material: &str,
        fragment_source: &str,
        encode: impl Fn(&super::PostProcessInput<'_>) -> T + 'static,
    ) -> prism::RenderResult<prism::frame::v2::PostProcessId> {
        self.post_process.check_material(material)?;
        let id = self.register_post_effect(
            assets,
            context.surface_format(),
            material,
            fragment_source,
            std::mem::size_of::<T>() as u64,
        )?;
        self.post_process.register(material.to_owned(), id, encode);
        Ok(id)
    }

    /// Historical add / write / enable ergonomics, backed by V2 fullscreen passes.
    pub fn add_post_process_pass<T: bytemuck::Pod>(
        &mut self,
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
        fragment_source: &str,
        uniform: Option<T>,
    ) -> prism::RenderResult<prism::frame::v2::PostProcessId> {
        let bytes = uniform
            .map(|uniform| bytemuck::bytes_of(&uniform).to_vec())
            .unwrap_or_default();
        let id = self.register_post_effect(
            assets,
            context.surface_format(),
            "post-process pass",
            fragment_source,
            bytes.len() as u64,
        )?;
        self.post_process.add_manual(id, bytes);
        Ok(id)
    }

    pub fn enable_post_process_pass(
        &mut self,
        id: prism::frame::v2::PostProcessId,
    ) -> prism::RenderResult<()> {
        self.post_process.enable(id, true)
    }
    pub fn disable_post_process_pass(
        &mut self,
        id: prism::frame::v2::PostProcessId,
    ) -> prism::RenderResult<()> {
        self.post_process.enable(id, false)
    }
    pub fn write_post_process_uniform<T: bytemuck::Pod>(
        &mut self,
        id: prism::frame::v2::PostProcessId,
        uniform: T,
    ) -> prism::RenderResult<()> {
        self.post_process.write(id, uniform)
    }

    pub fn register_post_effect(
        &mut self,
        assets: &mut RenderAssets,
        format: wgpu::TextureFormat,
        label: &str,
        fragment_source: &str,
        parameter_size: u64,
    ) -> prism::RenderResult<prism::frame::v2::PostProcessId> {
        use prism::{
            material::{MaterialTemplate, ShaderEntry},
            render::FullscreenEffect,
        };
        if parameter_size % 4 != 0 {
            return Err(prism::RenderError::new(
                "post-process uniform",
                "uniform size must be a multiple of four bytes",
            ));
        }
        let vertex = assets.gpu.create_shader(
            "post vertex",
            include_str!("../graphic_data/shader/default_post_process.vert.wgsl"),
        )?;
        let fragment = assets.gpu.create_shader(label, fragment_source)?;
        let mut template = MaterialTemplate::fullscreen(label, vertex, format);
        template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
        let mut schema = FullscreenEffect::source_schema();
        if parameter_size > 0 {
            schema.groups.push(super::assets::uniform_group(
                1,
                parameter_size,
                wgpu::ShaderStages::FRAGMENT,
            ));
        }
        template.schema = schema;
        let template = assets.gpu.create_template(template)?;
        Ok(self.graph.register_post_process(FullscreenEffect {
            template,
            source_binding: (0, 0),
            sampler_binding: (0, 1),
            parameter_binding: (parameter_size > 0).then_some((1, 0)),
            bindings: vec![],
        }))
    }

    pub fn new(
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
    ) -> prism::RenderResult<Self> {
        fn camera(assets: &mut RenderAssets) -> prism::RenderResult<(BufferId, BindingSetHandle)> {
            let buffer = assets.gpu.create_buffer(
                "view matrix",
                64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            )?;
            let bindings = assets.gpu.create_bindings(BindingSet {
                group: 0,
                entries: vec![(
                    0,
                    BindingResource::Buffer {
                        handle: buffer,
                        offset: 0,
                        size: 64,
                    },
                )],
            });
            Ok((buffer, bindings))
        }
        Ok(Self {
            graph: GraphRenderer::from_context(context),
            pending: None,
            vertices: MeshBuffer::new(assets, wgpu::BufferUsages::VERTEX)?,
            indices: MeshBuffer::new(assets, wgpu::BufferUsages::INDEX)?,
            cameras: [camera(assets)?, camera(assets)?],
            geometries: vec![],
            materials: vec![],
            mesh: RawMesh::with_capacity(4096, 6144),
            text: TextOverlay::new(context),
            post_process: Default::default(),
        })
    }

    /// Keep only the freshest scene while waiting for a redraw.
    pub fn queue(&mut self, frame: SceneFrame) {
        self.pending = Some(frame);
    }
    pub fn take_frame(&mut self) -> Option<SceneFrame> {
        self.pending.take()
    }

    fn prepare(
        &mut self,
        assets: &mut RenderAssets,
        scene: &mut SceneFrame,
        width: u32,
        height: u32,
    ) -> prism::RenderResult<FrameV2> {
        for id in self.geometries.drain(..) {
            assets.gpu.remove_geometry(id);
        }
        for id in self.materials.drain(..) {
            assets.gpu.remove_material(id);
        }
        self.mesh.clear();
        let mut frame = FrameV2::default();
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
            frame.uploads.push(UploadRequest::Buffer {
                target: self.cameras[index].0,
                offset: 0,
                data: bytemuck::bytes_of(matrix).to_vec(),
            });
        }
        frame.views = vec![
            ViewSubmission {
                bindings: vec![self.cameras[0].1.into()],
                phases: vec![
                    PhaseSubmission {
                        phase: WORLD,
                        sort: SortPolicy::SubmissionOrder,
                    },
                    PhaseSubmission {
                        phase: VFX,
                        sort: SortPolicy::SubmissionOrder,
                    },
                ],
                viewport: None,
            },
            ViewSubmission {
                bindings: vec![self.cameras[1].1.into()],
                phases: vec![PhaseSubmission {
                    phase: RenderPhaseId::OVERLAY,
                    sort: SortPolicy::SubmissionOrder,
                }],
                viewport: None,
            },
        ];
        let mut batches: Vec<(RenderPhaseId, MaterialHandle, Range<u32>)> = vec![];
        for (phase, commands) in [
            (WORLD, &mut scene.world),
            (VFX, &mut scene.vfx),
            (RenderPhaseId::OVERLAY, &mut scene.hud),
        ] {
            // Stable painter order; changing texture must never reorder alpha draws.
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
                if let Some((previous_phase, previous_material, range)) = batches.last_mut()
                    && *previous_phase == phase
                    && *previous_material == material
                {
                    range.end = end;
                } else {
                    batches.push((phase, material, start..end));
                }
            }
        }
        let vertex_size = self.mesh.vertices_bytes().len() as u64;
        let index_size = self.mesh.indices_bytes().len() as u64;
        if index_size != 0 {
            self.vertices
                .reserve(assets, vertex_size, wgpu::BufferUsages::VERTEX)?;
            self.indices
                .reserve(assets, index_size, wgpu::BufferUsages::INDEX)?;
            frame.uploads.extend([
                UploadRequest::Buffer {
                    target: self.vertices.id,
                    offset: 0,
                    data: self.mesh.vertices_bytes().to_vec(),
                },
                UploadRequest::Buffer {
                    target: self.indices.id,
                    offset: 0,
                    data: self.mesh.indices_bytes().to_vec(),
                },
            ]);
            for (phase, material, elements) in batches {
                let geometry = assets.gpu.create_geometry(Geometry {
                    vertices: vec![VertexBuffer {
                        slot: 0,
                        buffer: self.vertices.id,
                        range: 0..vertex_size,
                    }],
                    indices: Some(IndexBuffer {
                        buffer: self.indices.id,
                        range: 0..index_size,
                        format: wgpu::IndexFormat::Uint32,
                    }),
                    elements,
                    base_vertex: 0,
                });
                self.geometries.push(geometry);
                frame.objects.push(RenderObject {
                    geometry,
                    material,
                    instances: 0..1,
                    object_bindings: vec![],
                    phase,
                    sort_key: SortKey(frame.objects.len() as u64),
                    bounds: None,
                });
            }
        }
        frame.post_process = self
            .post_process
            .submissions(&scene.post_effects, [width, height]);
        frame.post_process.append(&mut scene.post_process);
        Ok(frame)
    }

    fn tessellate(
        &mut self,
        assets: &mut RenderAssets,
        command: &Primitive,
    ) -> prism::RenderResult<Option<MaterialHandle>> {
        let (shape, material) = match command {
            Primitive::Shape { shape, blend, .. } => {
                Tesselator::tesselate(shape, &mut self.mesh);
                return Ok(Some(assets.shapes[blend]));
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
                blend,
                ..
            } => {
                let bindings = if let Some(texture) = texture_id {
                    let material = assets.sprite_material(*texture, *blend, false)?;
                    assets.gpu.material(material)?.bindings.clone()
                } else {
                    vec![]
                };
                let material = assets.gpu.create_material(Material {
                    template: *template,
                    bindings,
                    parameters: (!uniform_data.is_empty()).then(|| ParameterBlock {
                        group: 2,
                        binding: 0,
                        bytes: uniform_data.clone(),
                    }),
                })?;
                self.materials.push(material);
                (
                    Shape::Quad {
                        pos: *pos,
                        size: *size,
                        rotation: *rotation,
                        uv: *uv,
                        color: *tint,
                    },
                    material,
                )
            }
            Primitive::Text { .. } => return Ok(None),
        };
        Tesselator::tesselate(&shape, &mut self.mesh);
        Ok(Some(material))
    }

    pub fn render_to_texture(
        &mut self,
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
        mut scene: SceneFrame,
        target: &wgpu::Texture,
    ) -> prism::RenderResult<prism::render::FrameReport> {
        let frame = self.prepare(assets, &mut scene, target.width(), target.height())?;
        let report = self
            .graph
            .render_to_texture(&mut assets.gpu, frame, target)?;
        let mut encoder = context.create_encoder("HUD text overlay");
        self.text.render(
            context,
            &mut encoder,
            &target.create_view(&Default::default()),
            [target.width(), target.height()],
            &scene.hud,
        )?;
        context.submit(encoder);
        Ok(report)
    }

    pub fn render(
        &mut self,
        context: &mut prism::GpuContext,
        assets: &mut RenderAssets,
        scene: SceneFrame,
    ) -> prism::RenderResult<Option<OverlayContext>> {
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
                return Err(prism::RenderError::new("surface", "validation error"));
            }
            _ => return Ok(None),
        };
        let report = self.render_to_texture(context, assets, scene, &surface.texture)?;
        for error in &report.skipped_objects {
            tracing::error!(%error, "Objet ignoré par Prism V2");
        }
        let overlay = OverlayContext {
            surface_view: surface.texture.create_view(&Default::default()),
            surface_texture: surface,
            encoder: context.create_encoder("surface overlays"),
            size: (context.size.width, context.size.height),
        };
        Ok(Some(overlay))
    }
}

use std::sync::Arc;
use utils::ids::{BufferId, ShaderId};

use crate::{
    context::GpuContext,
    draw::{commands::DrawCommand, text::TextRenderer},
    errors::PassError,
    geometry::{mesh::RawMesh, shape::Shape, tesselator::Tesselator},
    pass::{HudInput, Pass},
    resource::{
        buffer::GpuBufferManager,
        pipeline::{BlendMode, PipelineKey, PipelineManager, VertexFormat},
        shader::ShaderManager,
    },
};

pub struct HudPass {
    vert_shader: ShaderId,
    frag_shader: ShaderId,
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    vertex_buffer_size: u64,
    index_buffer_size: u64,
    index_count: u32,
    mesh: RawMesh,
    pipeline: Arc<wgpu::RenderPipeline>,
    text_renderer: TextRenderer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl HudPass {
    pub fn new(
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        pipelines: &mut PipelineManager,
        shaders: &ShaderManager,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
        surface_format: wgpu::TextureFormat,
    ) -> Result<Self, PassError> {
        let _span = tracing::info_span!("HudPass::new").entered();

        let index_buffer_size = 1024 * 12;
        let vertex_buffer_size = 1024 * 64;

        let index_buffer = buffers.create_buffer(
            ctx,
            Some("HudPass Index Buffer"),
            index_buffer_size,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        )?;

        let vertex_buffer = buffers.create_buffer(
            ctx,
            Some("HudPass Vertex Buffer"),
            vertex_buffer_size,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        )?;

        let mesh = RawMesh::with_capacity(1024, 3072);
        let index_count = mesh.indices().len() as u32;

        let pipeline_key = PipelineKey {
            vertex_shader: vert_shader,
            fragment_shader: frag_shader,
            blend_mode: BlendMode::Alpha,
            vertex_format: VertexFormat::Pos2UvColor,
            bind_groups: &crate::CAM_BIND_GROUP[0..1],
        };

        let pipeline = pipelines.get_or_create(ctx, shaders, pipeline_key.clone())?;

        let camera_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HudPass Camera Uniform Buffer"),
            size: std::mem::size_of::<utils::math::Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_renderer = TextRenderer::new(ctx, surface_format);

        let layouts = pipelines
            .get_layouts(&pipeline_key)
            .ok_or_else(|| {
                tracing::error!("Impossible de récupérer les BindGroupLayouts pour la HudPass");
                PassError::LayoutsNotFound
            })?;

        let camera_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HudPass Camera BindGroup"),
            layout: &layouts[0],
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        tracing::info!("Passe de rendu HudPass initialisée avec succès");

        Ok(Self {
            vert_shader,
            frag_shader,
            vertex_buffer,
            index_buffer,
            vertex_buffer_size,
            index_buffer_size,
            index_count,
            mesh,
            pipeline,
            text_renderer,
            camera_buffer,
            camera_bind_group,
        })
    }

    pub fn text_renderer(&self) -> &TextRenderer {
        &self.text_renderer
    }

    pub fn set_shader(
        &mut self,
        ctx: &GpuContext,
        pipelines: &mut PipelineManager,
        shaders: &ShaderManager,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), PassError> {
        let pipeline = pipelines.get_or_create(
            ctx,
            shaders,
            PipelineKey {
                vertex_shader: vert_shader,
                fragment_shader: frag_shader,
                blend_mode: BlendMode::Alpha,
                vertex_format: VertexFormat::Pos2UvColor,
                bind_groups: &crate::CAM_BIND_GROUP[0..1],
            },
        )?;

        self.vert_shader = vert_shader;
        self.frag_shader = frag_shader;
        self.pipeline = pipeline;

        tracing::debug!(vs = %vert_shader, fs = %frag_shader, "Shaders de la HudPass mis à jour");
        Ok(())
    }
}

impl Pass for HudPass {
    type Input<'a> = HudInput<'a>;

    fn prepare<'a>(
        &mut self,
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        input: &mut Self::Input<'a>,
    ) {
        let _span = tracing::trace_span!("HudPass::prepare").entered();

        self.mesh.clear();

        for cmd in input.commands.commands() {
            match cmd {
                DrawCommand::Shape { shape, .. } => Tesselator::tesselate(shape, &mut self.mesh),
                DrawCommand::Texture {
                    pos,
                    size,
                    rotation,
                    uv,
                    tint,
                    ..
                } => {
                    Tesselator::tesselate(
                        &Shape::Quad {
                            pos: *pos,
                            size: *size,
                            rotation: *rotation,
                            color: *tint,
                            uv: *uv,
                        },
                        &mut self.mesh,
                    );
                }
                DrawCommand::Mesh { mesh, .. } => {
                    self.mesh.append(mesh);
                }
                _ => (),
            }
        }

        ctx.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[input.camera]),
        );

        if let Err(err) = self.text_renderer.prepare(ctx, input.commands.commands()) {
            tracing::error!("Échec de la préparation du texte dans HudPass : {err}");
        }

        let required_vertex_bytes =
            self.mesh.vertices().len() as u64 * std::mem::size_of::<crate::geometry::mesh::Vertex>() as u64;

        if required_vertex_bytes > self.vertex_buffer_size {
            self.vertex_buffer_size = (self.vertex_buffer_size * 2).max(required_vertex_bytes);
            tracing::info!(
                new_size = self.vertex_buffer_size,
                "Agrandissement du Vertex Buffer dans HudPass"
            );

            let former_buffer = self.vertex_buffer;
            if let Ok(new_buf) = buffers.create_buffer(
                ctx,
                Some("HudPass Vertex Buffer (Resized)"),
                self.vertex_buffer_size,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ) {
                self.vertex_buffer = new_buf;
                let _ = buffers.remove(former_buffer);
            }
        }
        
        let required_index_bytes =
            self.mesh.indices().len() as u64 * std::mem::size_of::<u32>() as u64;

        if required_index_bytes > self.index_buffer_size {
            self.index_buffer_size = (self.index_buffer_size * 2).max(required_index_bytes);
            tracing::info!(
                new_size = self.index_buffer_size,
                "Agrandissement de l'Index Buffer dans HudPass"
            );

            let former_buffer = self.index_buffer;
            if let Ok(new_buf) = buffers.create_buffer(
                ctx,
                Some("HudPass Index Buffer (Resized)"),
                self.index_buffer_size,
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            ) {
                self.index_buffer = new_buf;
                let _ = buffers.remove(former_buffer);
            }
        }

        self.index_count = self.mesh.indices().len() as u32;

        if let Err(err) = buffers.write_buffer(ctx, self.vertex_buffer, self.mesh.vertices_bytes()) {
            tracing::error!("Échec d'écriture dans le Vertex Buffer de HudPass : {err}");
        }

        if let Err(err) = buffers.write_buffer(ctx, self.index_buffer, self.mesh.indices_bytes()) {
            tracing::error!("Échec d'écriture dans l'Index Buffer de HudPass : {err}");
        }
    }

    fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        buffers: &GpuBufferManager,
    ) {
        let mut hud_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hud Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        if self.index_count > 0 {
            let index_buffer = buffers.get(self.index_buffer);
            let vertex_buffer = buffers.get(self.vertex_buffer);

            match (index_buffer, vertex_buffer) {
                (Some(idx_buf), Some(vtx_buf)) => {
                    hud_render_pass.set_pipeline(&self.pipeline);
                    hud_render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    hud_render_pass
                        .set_index_buffer(idx_buf.buffer.slice(..), wgpu::IndexFormat::Uint32);
                    hud_render_pass.set_vertex_buffer(0, vtx_buf.buffer.slice(..));
                    hud_render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                }
                _ => {
                    tracing::error!("Vertex ou Index Buffer introuvable dans HudPass lors de l'exécution");
                }
            }
        }

        if let Err(err) = self.text_renderer.render(&mut hud_render_pass) {
            tracing::error!("Échec du rendu du texte dans HudPass : {err}");
        }
    }
}
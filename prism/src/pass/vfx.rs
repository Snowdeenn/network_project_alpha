use crate::{
    context::GpuContext,
    draw::commands::DrawCommand,
    geometry::{mesh::RawMesh, shape::Shape, tesselator::Tesselator},
    pass::{Pass, VfxInput},
    resource::{
        buffer::GpuBufferManager,
        pipeline::{BlendMode, PipelineKey, PipelineManager, VertexFormat},
        shader::ShaderManager,
    },
};
use std::sync::Arc;
use utils::ids::{BufferId, ShaderId};
pub struct VfxPass {
    vert_shader: ShaderId,
    frag_shader: ShaderId,
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    vertex_buffer_size: u64,
    index_buffer_size: u64,
    index_count: u32,
    mesh: RawMesh,
    pipeline: Arc<wgpu::RenderPipeline>,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl VfxPass {
    pub fn new(
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        pipelines: &mut PipelineManager,
        shaders: &mut ShaderManager,
        vertex_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Self {
        let index_buffer_size = 1024 * 12;
        let vertex_buffer_size = 1024 * 64;
        let index_buffer = buffers.create_buffer(
            ctx,
            index_buffer_size,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        );
        let vertex_buffer = buffers.create_buffer(
            ctx,
            vertex_buffer_size,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let camera_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform"),
            size: std::mem::size_of::<utils::math::Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mesh = RawMesh::with_capacity(1024, 3072);
        let index_count = mesh.indices().len() as u32;
        let pipeline_key = PipelineKey {
            vertex_shader,
            fragment_shader: frag_shader,
            blend_mode: BlendMode::Additive,
            vertex_format: VertexFormat::Pos2UvColor,
            bind_groups: crate::pass::DEFAULT_BIND_GROUPS, // Todo: Changer l'emplacement du world bind group
        };
        let pipeline = pipelines.get_or_create(ctx, shaders, pipeline_key.clone());
        let layouts = pipelines.get_layouts(&pipeline_key).unwrap();
        let camera_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera BindGroup"),
            layout: &layouts[0],
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        Self {
            vert_shader: vertex_shader,
            frag_shader,
            vertex_buffer,
            index_buffer,
            vertex_buffer_size,
            index_buffer_size,
            index_count,
            mesh,
            pipeline,
            camera_buffer,
            camera_bind_group,
        }
    }
}

impl Pass for VfxPass {
    type Input<'a> = VfxInput<'a>;
    fn prepare<'a>(
        &mut self,
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        input: &Self::Input<'a>,
    ) {
        self.mesh.clear();
        ctx.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[input.camera]),
        );
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
        if self.mesh.vertices().len() as u64 > self.vertex_buffer_size {
            self.vertex_buffer_size *= 2;
            let former_buffer = self.vertex_buffer;
            self.vertex_buffer = buffers.create_buffer(
                ctx,
                self.vertex_buffer_size,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            );
            buffers.remove(former_buffer);
        }
        if self.mesh.indices().len() as u64 > self.index_buffer_size {
            self.index_buffer_size *= 2;
            let former_buffer = self.index_buffer;
            self.index_buffer = buffers.create_buffer(
                ctx,
                self.index_buffer_size,
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            );
            buffers.remove(former_buffer);
        }
        self.index_count = self.mesh.indices().len() as u32;
        buffers.write_buffer(ctx, self.vertex_buffer, self.mesh.vertices_bytes());
        buffers.write_buffer(ctx, self.index_buffer, self.mesh.indices_bytes());
    }

    fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        buffers: &GpuBufferManager,
    ) {
        if self.index_count == 0 {
            return;
        }
        let mut vfx_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Vfx Render Pass"),
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
        let index_buffer = buffers
            .get(self.index_buffer)
            .expect("[GpuBufferManager] devrait retourner l'index buffer de la vfx pass");
        let vertex_buffer = buffers
            .get(self.vertex_buffer)
            .expect("[GpuBufferManager] devrait retourner le vertex buffer de la vfx pass");
        vfx_render_pass.set_pipeline(&self.pipeline);
        vfx_render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        vfx_render_pass.set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
        vfx_render_pass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
        vfx_render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

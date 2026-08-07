use crate::{
    context::GpuContext,
    draw::commands::DrawCommand,
    geometry::{mesh::RawMesh, shape::Shape, tesselator::Tesselator},
    pass::{Pass, WorldInput},
    resource::{
        buffer::GpuBufferManager,
        pipeline::{BlendMode, PipelineKey, PipelineManager, VertexFormat},
        shader::ShaderManager,
    },
};
use std::sync::Arc;
use utils::ids::{BufferId, ShaderId};

pub struct TextureBatch {
    index_offset: u32,
    index_count: u32,
    bind_group: wgpu::BindGroup,
}

pub struct WorldPass {
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
    batches: Vec<TextureBatch>,
    texture_group_layout: wgpu::BindGroupLayout,
}

impl WorldPass {
    pub fn new(
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        pipelines: &mut PipelineManager,
        shaders: &mut ShaderManager,
        vert_shader: ShaderId,
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
            vertex_shader: vert_shader,
            fragment_shader: frag_shader,
            blend_mode: BlendMode::Alpha,
            vertex_format: VertexFormat::Pos2UvColor,
            bind_groups: &crate::TEXTURE_BIND_GROUP,
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
        let batches = Vec::with_capacity(4096);
        let texture_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Texture Bind Group Layout World pass"),
                    entries: &[
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
                });
        Self {
            vert_shader,
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
            batches,
            texture_group_layout,
        }
    }
    pub fn set_shader(
        &mut self,
        ctx: &GpuContext,
        pipelines: &mut PipelineManager,
        shaders: &ShaderManager,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) {
        let pipeline = pipelines.get_or_create(
            ctx,
            shaders,
            PipelineKey {
                vertex_shader: vert_shader,
                fragment_shader: frag_shader,
                blend_mode: BlendMode::Alpha,
                vertex_format: VertexFormat::Pos2UvColor,
                bind_groups: &crate::TEXTURE_BIND_GROUP,
            },
        );
        self.pipeline = pipeline;
    }

    fn create_texture_bind_group(
        &self,
        ctx: &GpuContext,
        texture_id: utils::ids::TextureId,
        textures: &crate::TextureManager,
    ) -> wgpu::BindGroup {
        let gpu_texture = textures
            .get(texture_id)
            .expect("[WorldPass] TextureId invalide lors de la création du BindGroup");

        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture BindGroup — WorldPass batch"),
            layout: &self.texture_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gpu_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu_texture.sampler),
                },
            ],
        })
    }
}

impl Pass for WorldPass {
    type Input<'a> = WorldInput<'a>;
    fn prepare<'a>(
        &mut self,
        ctx: &GpuContext,
        buffers: &mut GpuBufferManager,
        input: &mut Self::Input<'a>,
    ) {
        self.mesh.clear();
        self.batches.clear();
        ctx.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[input.camera]),
        );
        let white_id = input.texture.white_texture();
        let get_cmd_texture_id = |cmd: &DrawCommand| -> Option<utils::ids::TextureId> {
            match cmd {
                DrawCommand::Texture { id, .. } => Some(*id),
                DrawCommand::Text { .. } => None,
                _ => Some(white_id),
            }
        };
        let get_sort_key = |cmd: &DrawCommand| -> (u8, usize) {
            match cmd {
                DrawCommand::Texture { id, layer, .. } => (*layer, id.index),
                DrawCommand::Shape { layer, .. } => (*layer, white_id.index),
                DrawCommand::Mesh { layer, .. } => (*layer, white_id.index),
                DrawCommand::Text { layer, .. } => (*layer, usize::MAX),
            }
        };

        input
            .commands
            .commands_mut()
            .sort_by_key(|cmd| get_sort_key(cmd));

        let mut current_texture_id: Option<utils::ids::TextureId> = None;
        let mut batch_index_start = 0u32;

        for cmd in input.commands.commands() {
            let tex_id = get_cmd_texture_id(cmd);
            if current_texture_id != tex_id {
                if let Some(prev_id) = current_texture_id {
                    let index_count = self.mesh.indices().len() as u32 - batch_index_start;
                    if index_count > 0 {
                        let bind_group =
                            self.create_texture_bind_group(ctx, prev_id, input.texture);
                        self.batches.push(TextureBatch {
                            index_offset: batch_index_start,
                            index_count,
                            bind_group,
                        });
                    }
                }
                current_texture_id = tex_id;
                batch_index_start = self.mesh.indices().len() as u32;
            }
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
        if let Some(last_id) = current_texture_id {
            let index_count = self.mesh.indices().len() as u32 - batch_index_start;
            if index_count > 0 {
                let bind_group = self.create_texture_bind_group(ctx, last_id, input.texture);
                self.batches.push(TextureBatch {
                    index_offset: batch_index_start,
                    index_count,
                    bind_group,
                });
            }
        }
        if self.mesh.vertices().len() as u64
            * std::mem::size_of::<crate::geometry::mesh::Vertex>() as u64
            > self.vertex_buffer_size
        {
            self.vertex_buffer_size *= 2;
            let former_buffer = self.vertex_buffer;
            self.vertex_buffer = buffers.create_buffer(
                ctx,
                self.vertex_buffer_size,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            );
            buffers.remove(former_buffer);
        }
        if self.mesh.indices().len() as u64 * std::mem::size_of::<u32>() as u64
            > self.index_buffer_size
        {
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
        let mut world_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("World Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
            .expect("[GpuBufferManager] devrait retourner l'index buffer de la WorldPass");
        let vertex_buffer = buffers
            .get(self.vertex_buffer)
            .expect("[GpuBufferManager] devrait retourner le vertex buffer de la WorldPass");
        world_render_pass.set_pipeline(&self.pipeline);
        world_render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        world_render_pass
            .set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
        world_render_pass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));

        for batch in &self.batches {
            world_render_pass.set_bind_group(1, &batch.bind_group, &[]);
            world_render_pass.draw_indexed(
                batch.index_offset..batch.index_offset + batch.index_count,
                0,
                0..1,
            );
        }
    }
}

use crate::GpuBufferManager;
use crate::ShaderManager;
use crate::context::GpuContext;
use crate::pass::{Pass, PostProcessInput};
use std::sync::Arc;
use utils::ids::ShaderId;

pub struct PostProcessPass {
    pipeline: Arc<wgpu::RenderPipeline>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
}

impl PostProcessPass {
    pub fn new(
        ctx: &GpuContext,
        shaders: &mut ShaderManager,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("PostProcess BindGroupLayout"),
                    entries: &[
                        // texture
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
                        // sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            anisotropy_clamp: 1,
            compare: None,
            border_color: None,
        });

        let pipeline =
            ctx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("PostProcess Pipeline"),
                    layout: Some(&ctx.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[Some(&bind_group_layout)],
                            immediate_size: 0,
                        },
                    )),
                    vertex: wgpu::VertexState {
                        module: &shaders.get(vert_shader).unwrap().module,
                        entry_point: Some("vs_main"),
                        buffers: &[], // pas de vertex buffer
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shaders.get(frag_shader).unwrap().module,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                });
        Self {
            pipeline: Arc::new(pipeline),
            bind_group_layout,
            bind_group: None,
            sampler,
        }
    }
}

impl Pass for PostProcessPass {
    type Input<'a> = PostProcessInput<'a>;
    fn prepare<'a>(
        &mut self,
        ctx: &GpuContext,
        _buffers: &mut GpuBufferManager,
        input: &mut Self::Input<'a>,
    ) {
        self.bind_group = Some(ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PostProcess BindGroup"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input.source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
    }

    fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _buffers: &GpuBufferManager,
    ) {
        let Some(bind_group) = &self.bind_group else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("PostProcess Pass"),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1); // fullscreen triangle — pas de vertex buffer
    }
}

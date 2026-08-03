use std::collections::HashMap;
use std::sync::Arc;
use utils::ids::ShaderId;
use wgpu::VertexBufferLayout;

use crate::{context::GpuContext, resource::shader::ShaderManager};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutEntryKey {
    pub binding: u32,
    pub visibility: wgpu::ShaderStages,
    pub ty: BindingTypeKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindingTypeKey {
    UniformBuffer,
    Texture2D,
    Sampler,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub vertex_shader: ShaderId,
    pub fragment_shader: ShaderId,
    pub blend_mode: BlendMode,
    pub vertex_format: VertexFormat,
    pub bind_groups: &'static [&'static [BindGroupLayoutEntryKey]],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Alpha,    // rendu normal avec transparence
    Additive, // glow, VFX lumineux
    Multiply, // ombres, effets sombres
    Opaque,   // pas de transparence — le plus rapide
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    Pos2Color,   // [Vec2 pos, Vec4 color] — géométrie simple
    Pos2UvColor, // [Vec2 pos, Vec2 uv, Vec4 color] — sprites
}

pub struct PipelineManager {
    pipelines: HashMap<PipelineKey, Arc<wgpu::RenderPipeline>>,
    layouts: HashMap<PipelineKey, Vec<wgpu::BindGroupLayout>>,
    surface_format: wgpu::TextureFormat,
}

impl PipelineManager {
    pub fn new(surface_format: wgpu::TextureFormat) -> Self {
        Self {
            pipelines: HashMap::new(),
            layouts: HashMap::new(),
            surface_format,
        }
    }

    pub fn get_or_create(
        &mut self,
        ctx: &GpuContext,
        shaders: &ShaderManager,
        key: PipelineKey,
    ) -> Arc<wgpu::RenderPipeline> {
        if !self.pipelines.contains_key(&key) {
            let layouts = Self::create_bind_group_layouts(ctx, key.bind_groups);
            let pipeline = self.create_pipeline(ctx, shaders, &key);
            self.layouts.insert(key.clone(), layouts);
            self.pipelines.insert(key.clone(), Arc::new(pipeline));
        }
        self.pipelines.get(&key).unwrap().clone()
    }

    pub fn invalidate_shader(&mut self, shader_id: ShaderId) {
        self.pipelines
            .retain(|key, _| key.vertex_shader != shader_id && key.fragment_shader != shader_id);
    }

    pub fn invalidate_all(&mut self) {
        self.pipelines.clear();
    }

    pub fn get_layouts(&self, key: &PipelineKey) -> Option<&[wgpu::BindGroupLayout]> {
        self.layouts.get(key).map(|v| v.as_slice())
    }

    fn create_pipeline(
        &self,
        ctx: &GpuContext,
        shaders: &ShaderManager,
        key: &PipelineKey,
    ) -> wgpu::RenderPipeline {
        let blend_mode = Self::blend_mode(key.blend_mode);
        let vertex_format = Self::vertex_layout(key.vertex_format);

        let vertex_shader = shaders.get(key.vertex_shader).unwrap();
        let frag_shader = shaders.get(key.fragment_shader).unwrap();

        let bind_group_layouts = Self::create_bind_group_layouts(ctx, key.bind_groups);
        let bind_group_layout_refs: Vec<Option<&wgpu::BindGroupLayout>> =
            bind_group_layouts.iter().map(Some).collect();

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &bind_group_layout_refs,
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader.module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(vertex_format)],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &frag_shader.module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_format,
                        blend: blend_mode,
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        pipeline
    }

    fn blend_mode(mode: BlendMode) -> Option<wgpu::BlendState> {
        match mode {
            BlendMode::Additive => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            BlendMode::Alpha => Some(wgpu::BlendState::ALPHA_BLENDING),
            BlendMode::Multiply => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::Zero,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::DstAlpha,
                    dst_factor: wgpu::BlendFactor::Zero,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            BlendMode::Opaque => None,
        }
    }

    fn vertex_layout(format: VertexFormat) -> wgpu::VertexBufferLayout<'static> {
        match format {
            VertexFormat::Pos2Color => VertexBufferLayout {
                array_stride: (size_of::<[f32; 2]>() + size_of::<[f32; 4]>()) as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: size_of::<[f32; 2]>() as u64,
                        shader_location: 1,
                    },
                ],
            },
            VertexFormat::Pos2UvColor => VertexBufferLayout {
                array_stride: (size_of::<[f32; 2]>()
                    + size_of::<[f32; 2]>()
                    + size_of::<[f32; 4]>()) as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: (size_of::<[f32; 2]>()) as u64,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: (size_of::<[f32; 2]>() + size_of::<[f32; 2]>()) as u64,
                        shader_location: 2,
                    },
                ],
            },
        }
    }

    fn create_bind_group_layouts(
        ctx: &GpuContext,
        groups: &[&[BindGroupLayoutEntryKey]],
    ) -> Vec<wgpu::BindGroupLayout> {
        groups
            .iter()
            .map(|entries| {
                let wgpu_entries: Vec<wgpu::BindGroupLayoutEntry> = entries
                    .iter()
                    .map(|e| wgpu::BindGroupLayoutEntry {
                        binding: e.binding,
                        visibility: e.visibility,
                        ty: match e.ty {
                            BindingTypeKey::UniformBuffer => wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            BindingTypeKey::Texture2D => wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            BindingTypeKey::Sampler => {
                                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                            }
                        },
                        count: None,
                    })
                    .collect();

                ctx.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: &wgpu_entries,
                    })
            })
            .collect()
    }
}

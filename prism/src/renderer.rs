use std::sync::Arc;

use crate::pass::Pass;

pub struct Renderer {
    ctx: crate::GpuContext,
    buffers: crate::GpuBufferManager,
    pipelines: crate::PipelineManager,
    shaders: crate::ShaderManager,
    textures: crate::TextureManager,
    world: crate::WorldPass,
    vfx: crate::VfxPass,
    hud: crate::HudPass,
    post_process_passes: Vec<crate::PostProcessPass>,
    intermediate_a: wgpu::Texture,
    intermediate_view_a: wgpu::TextureView,
    intermediate_b: wgpu::Texture,
    intermediate_view_b: wgpu::TextureView,
    current_source: DoubleBufferIndex,
    frame_manager: crate::FrameManager,
}

impl Renderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        vert_shader_path: &str,
        frag_shader_path: &str,
        post_vert_path: &str,
        post_frag_path: &str,
    ) -> Self {
        let ctx = pollster::block_on(crate::GpuContext::new(window)).unwrap();
        //let ctx = Arc::new(ctx);
        let mut buffers = crate::GpuBufferManager::new();
        let mut shaders = crate::ShaderManager::new();
        let textures = crate::TextureManager::new();
        let mut pipelines = crate::PipelineManager::new(ctx.surface_format());
        let vert_shader = shaders.load(&ctx, vert_shader_path).unwrap();
        let frag_shader = shaders.load(&ctx, frag_shader_path).unwrap();
        let post_vert_shader = shaders.load(&ctx, post_vert_path).unwrap();
        let post_frag_shader = shaders.load(&ctx, post_frag_path).unwrap();
        let (intermediate_a, intermediate_view_a) = Self::create_intermediate(&ctx);
        let (intermediate_b, intermediate_view_b) = Self::create_intermediate(&ctx);

        let world = crate::WorldPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
        );
        let vfx = crate::VfxPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
        );
        let hud = crate::HudPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
            ctx.surface_format(),
        );
        let post_process_passes = vec![crate::PostProcessPass::new(
            &ctx,
            &mut shaders,
            post_vert_shader,
            post_frag_shader,
            ctx.surface_format(),
        )];
        let frame_manager = crate::FrameManager::new();
        Self {
            ctx,
            buffers,
            pipelines,
            shaders,
            textures,
            world,
            vfx,
            hud,
            post_process_passes,
            intermediate_a,
            intermediate_view_a,
            intermediate_b,
            intermediate_view_b,
            current_source: DoubleBufferIndex::Primary,
            frame_manager,
        }
    }

    pub fn render(&mut self, frame: crate::Frame) {
        let surface_texture = match self.ctx.current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.ctx.reconfigure();
                return;
            }
            _ => return,
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.ctx.create_encoder("frame");

        let (_source, target) = match self.current_source {
            DoubleBufferIndex::Primary => (&self.intermediate_view_a, &self.intermediate_view_b),
            DoubleBufferIndex::Secondary => (&self.intermediate_view_b, &self.intermediate_view_a),
        };

        self.world.prepare(
            &self.ctx,
            &mut self.buffers,
            &crate::WorldInput {
                commands: &frame.world,
                camera: frame.camera,
            },
        );
        self.world.execute(&mut encoder, target, &self.buffers);

        self.vfx.prepare(
            &self.ctx,
            &mut self.buffers,
            &crate::VfxInput {
                commands: &frame.vfx,
            },
        );
        self.vfx.execute(&mut encoder, target, &self.buffers);

        for pass in &mut self.post_process_passes {
            self.current_source = self.current_source.swap();
            let (src, tgt) = match self.current_source {
                DoubleBufferIndex::Primary => {
                    (&self.intermediate_view_a, &self.intermediate_view_b)
                }
                DoubleBufferIndex::Secondary => {
                    (&self.intermediate_view_b, &self.intermediate_view_a)
                }
            };
            pass.prepare(
                &self.ctx,
                &mut self.buffers,
                &crate::PostProcessInput {
                    source: src,
                    target: tgt,
                },
            );
            pass.execute(&mut encoder, tgt, &self.buffers);
        }

        self.hud.prepare(
            &self.ctx,
            &mut self.buffers,
            &crate::HudInput {
                commands: &frame.hud,
            },
        );
        self.hud.execute(&mut encoder, &surface_view, &self.buffers);

        self.ctx.submit(encoder);
        self.ctx.present(surface_texture);
    }

    fn create_intermediate(ctx: &crate::GpuContext) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Intermediate Target"),
            size: wgpu::Extent3d {
                width: ctx.size.width,
                height: ctx.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn frame_manager(&self) -> &crate::FrameManager {
        &self.frame_manager
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
        let (intermediate_a, intermediate_view_a) = Self::create_intermediate(&self.ctx);
        let (intermediate_b, intermediate_view_b) = Self::create_intermediate(&self.ctx);
        self.intermediate_a = intermediate_a;
        self.intermediate_view_a = intermediate_view_a;
        self.intermediate_b = intermediate_b;
        self.intermediate_view_b = intermediate_view_b;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleBufferIndex {
    Primary,
    Secondary,
}

impl DoubleBufferIndex {
    pub fn swap(self) -> Self {
        match self {
            Self::Primary => Self::Secondary,
            Self::Secondary => Self::Primary,
        }
    }
}

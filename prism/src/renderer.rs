use std::sync::Arc;

use utils::ids::ShaderId;

use crate::pass::Pass;

pub struct Renderer {
    ctx: crate::GpuContext,
    buffers: crate::GpuBufferManager,
    pipelines: crate::PipelineManager,
    shaders: crate::ShaderManager,
    textures: crate::TextureManager,
    materials: crate::MaterialManager,
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
    ) -> crate::Result<Self> {
        let ctx = pollster::block_on(crate::GpuContext::new(window))?;
        let mut buffers = crate::GpuBufferManager::new();
        let mut shaders = crate::ShaderManager::new();
        let textures = crate::TextureManager::new(&ctx);
        let mut pipelines = crate::PipelineManager::new(ctx.surface_format());
        let vert_shader = shaders.load(&ctx, vert_shader_path)?;
        let frag_shader = shaders.load(&ctx, frag_shader_path)?;
        let post_vert_shader = shaders.load(&ctx, post_vert_path)?;
        let post_frag_shader = shaders.load(&ctx, post_frag_path)?;
        let (intermediate_a, intermediate_view_a) = Self::create_intermediate(&ctx);
        let (intermediate_b, intermediate_view_b) = Self::create_intermediate(&ctx);
        let materials = crate::MaterialManager::new();
        let world = crate::WorldPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
        )?;
        let vfx = crate::VfxPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
        )?;
        let hud = crate::HudPass::new(
            &ctx,
            &mut buffers,
            &mut pipelines,
            &mut shaders,
            vert_shader,
            frag_shader,
            ctx.surface_format(),
        )?;
        let post_process_passes = vec![crate::PostProcessPass::new(
            &ctx,
            &mut shaders,
            post_vert_shader,
            post_frag_shader,
            ctx.surface_format(),
        )?];
        let frame_manager = crate::FrameManager::new();
        Ok(Self {
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
            materials,
        })
    }

    pub fn render(&mut self, mut frame: crate::Frame) {
        let surface_texture = match self.ctx.current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                tracing::warn!("Surface GPU suboptimale, reconfiguration au prochain cycle");
                self.ctx.reconfigure();
                t
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                tracing::trace!("Fenêtre occultée ou minimisée, rendu ignoré");
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                tracing::warn!("Surface GPU obsolète ou perdue, redimensionnement...");
                let size = self.ctx.size;
                self.ctx.resize(size.width, size.height);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                tracing::warn!("Timeout lors de la récupération de la frame (frame sautée)");
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                tracing::error!("Erreur de validation lors de la récupération de la texture");
                return;
            }
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.current_source = DoubleBufferIndex::Primary;
        let mut encoder = self.ctx.create_encoder("frame");

        let (_source, target) = match self.current_source {
            DoubleBufferIndex::Primary => (&self.intermediate_view_a, &self.intermediate_view_b),
            DoubleBufferIndex::Secondary => (&self.intermediate_view_b, &self.intermediate_view_a),
        };
        let screen_w = self.ctx.size.width;
        let screen_h = self.ctx.size.height;
        let cam_matrix = build_camera_matrix(
            frame.camera_pos,
            frame.cam_shake_offset,
            screen_w as f32,
            screen_h as f32,
        );

        self.world.prepare(
            &self.ctx,
            &mut self.buffers,
            &mut crate::WorldInput {
                commands: &mut frame.world,
                camera: cam_matrix,
                texture: &mut self.textures,
            },
        );
        self.world.execute(&mut encoder, target, &self.buffers, &self.materials);

        self.vfx.prepare(
            &self.ctx,
            &mut self.buffers,
            &mut crate::VfxInput {
                commands: &frame.vfx,
                camera: cam_matrix,
            },
        );
        self.vfx.execute(&mut encoder, target, &self.buffers, &self.materials);

        let last = self.post_process_passes.len().saturating_sub(1);
        for i in 0..self.post_process_passes.len() {
            self.current_source = self.current_source.swap();
            let (src, tgt) = match self.current_source {
                DoubleBufferIndex::Primary => {
                    (&self.intermediate_view_a, &self.intermediate_view_b)
                }
                DoubleBufferIndex::Secondary => {
                    (&self.intermediate_view_b, &self.intermediate_view_a)
                }
            };

            let render_target = if i == last { &surface_view } else { tgt };

            self.post_process_passes[i].prepare(
                &self.ctx,
                &mut self.buffers,
                &mut crate::PostProcessInput {
                    source: src,
                    target: tgt,
                },
            );
            self.post_process_passes[i].execute(&mut encoder, render_target, &self.buffers, &self.materials);
        }

        self.hud.prepare(
            &self.ctx,
            &mut self.buffers,
            &mut crate::HudInput {
                commands: &mut frame.hud,
                camera: cam_matrix,
                texture: &self.textures,
            },
        );
        self.hud.execute(&mut encoder, &surface_view, &self.buffers, &self.materials);

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
            format: ctx.surface_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
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

    pub fn load_shader(&mut self, path: &str) -> crate::Result<utils::ids::ShaderId> {
        Ok(self.shaders.load(&self.ctx, path)?)
    }
    pub fn load_texture(&mut self, path: &str) -> crate::Result<utils::ids::TextureId> {
        Ok(self.textures.load(&self.ctx, path)?)
    }

    pub fn screen_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.ctx.size
    }

    pub fn frame_manager(&self) -> &crate::FrameManager {
        &self.frame_manager
    }

    pub fn texture_mut(&mut self) -> &mut crate::TextureManager {
        &mut self.textures
    }
    pub fn texture(&self) -> &crate::TextureManager {
        &self.textures
    }
    pub fn shader_mut(&mut self) -> &mut crate::ShaderManager {
        &mut self.shaders
    }
    pub fn shader(&self) -> &crate::ShaderManager {
        &self.shaders
    }
    pub fn ctx(&self) -> &crate::GpuContext {
        &self.ctx
    }
    pub fn ctx_mut(&mut self) -> &mut crate::GpuContext {
        &mut self.ctx
    }
    pub fn pipeline_mut(&mut self) -> &mut crate::PipelineManager {
        &mut self.pipelines
    }
    pub fn pipeline(&self) -> &crate::PipelineManager {
        &self.pipelines
    }

    pub fn material_mut(&mut self) -> &mut crate::MaterialManager {
        &mut self.materials
    }
    pub fn material(&self) -> &crate::MaterialManager {
        &self.materials
    }
    pub fn set_world_shaders(
        &mut self,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), crate::PrismError> {
        self.world.set_shader(
            &self.ctx,
            &mut self.pipelines,
            &self.shaders,
            vert_shader,
            frag_shader,
        )?;
        Ok(())
    }

    pub fn set_vfx_shaders(
        &mut self,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), crate::PrismError> {
        self.vfx.set_shader(
            &self.ctx,
            &mut self.pipelines,
            &self.shaders,
            vert_shader,
            frag_shader,
        )?;
        Ok(())
    }

    pub fn set_hud_shaders(
        &mut self,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), crate::PrismError> {
        self.hud.set_shader(
            &self.ctx,
            &mut self.pipelines,
            &self.shaders,
            vert_shader,
            frag_shader,
        )?;
        Ok(())
    }

    pub fn set_post_process_shaders(
        &mut self,
        index: usize,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), crate::PrismError> {
        if let Some(pass) = self.post_process_passes.get_mut(index) {
            pass.set_shader(&self.ctx, &self.shaders, vert_shader, frag_shader)?;
            Ok(())
        } else {
            tracing::error!(
                index,
                total_passes = self.post_process_passes.len(),
                "Impossible de changer les shaders : index de post-process hors limites"
            );
            Ok(())
        }
    }

    pub fn add_post_process_pass(
        &mut self,
        vert_shader: ShaderId,
        frag_shader: ShaderId,
    ) -> Result<(), crate::PrismError> {
        let pass = crate::PostProcessPass::new(
            &self.ctx,
            &self.shaders,
            vert_shader,
            frag_shader,
            self.ctx.surface_format(),
        )?;
        self.post_process_passes.push(pass);
        Ok(())
    }

    pub fn ctx_and_textures_mut(&mut self) -> (&crate::GpuContext, &mut crate::TextureManager) {
        (&self.ctx, &mut self.textures)
    }

    pub fn create_pipeline(
        &mut self,
        key: crate::PipelineKey,
    ) -> Result<Arc<wgpu::RenderPipeline>, crate::PassError> {
        Ok(self.pipelines.get_or_create(&self.ctx, &self.shaders, key)?)
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

fn build_camera_matrix(
    pos: utils::math::Vec2,
    shake: utils::math::Vec2,
    screen_w: f32,
    screen_h: f32,
) -> utils::math::Mat4 {
    let proj = utils::math::Mat4::orthographic(0.0, screen_w, screen_h, 0.0, -1.0, 1.0);
    let view = utils::math::Mat4::translation(
        -pos.x + (screen_w * 0.5) + shake.x,
        -pos.y + (screen_h * 0.5) + shake.y,
        0.0,
    );
    proj.multiply(view)
}

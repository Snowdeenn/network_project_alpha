use utils::{
    arena::Arena,
    ids::{TextureId, TextureTag},
};

use crate::context::GpuContext;

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
}

pub struct TextureManager {
    textures: Arena<GpuTexture, TextureTag>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            textures: Arena::new(),
        }
    }

    pub fn load(&mut self, ctx: &GpuContext, path: &str) -> Option<TextureId> {
        let image = image::open(path).ok()?.to_rgba8();
        let (width, height) = image.dimensions();
        let bytes = image.as_raw();

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(path),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            mip_level_count: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Some(self.textures.insert(GpuTexture {
            texture,
            view,
            sampler,
            size: (width, height),
        }))
    }

    pub fn register(&mut self, texture: GpuTexture) -> TextureId {
        self.textures.insert(texture)
    }

    pub fn remove(&mut self, id: TextureId) -> Option<GpuTexture> {
        self.textures.remove(id)
    }

    pub fn get(&self, id: TextureId) -> Option<&GpuTexture> {
        self.textures.get(id)
    }

    pub fn get_mut(&mut self, id: TextureId) -> Option<&mut GpuTexture> {
        self.textures.get_mut(id)
    }
}

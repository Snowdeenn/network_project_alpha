use super::{AssetCache, TextureId};
use std::path::Path;
use velum::{
    RenderData,
    backend::{self as gpu, RenderError, RenderResult, texture},
};

pub fn load_cached(
    cache: &mut AssetCache,
    data: &mut RenderData,
    path: impl AsRef<Path>,
) -> Result<TextureId, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if let Some(&(id, owner)) = cache.textures.get(path)
        && data.get(owner).is_some()
    {
        return Ok(id);
    }
    let context = cache.context;
    let image = image::open(path)?.into_rgba8();
    let (width, height) = image.dimensions();
    // Velum's file loader uses Rgba8Unorm; keep sRGB decoding for Alpha assets.
    let mut builder = texture::TextureBuilder::new()
        .with_size(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        })
        .with_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .with_view_formats(&[]);
    if let Some(label) = path.to_str() {
        builder = builder.with_label(label);
    }
    let raw = builder.build_in(context);
    gpu::context::queue(context).write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &raw,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        raw.size(),
    );
    let (owner, _) = texture::register_texture(context, raw);
    let owner = owner.share();
    let id = owner.id();
    cache
        .textures
        .insert(path.to_owned(), (id, data.insert(owner)));
    Ok(id)
}

pub fn size(id: TextureId) -> RenderResult<(u32, u32)> {
    let resource =
        texture::get(id).ok_or_else(|| RenderError::new("texture", "missing texture"))?;
    let texture = match resource.as_ref() {
        texture::TextureResource::Separate(resource)
        | texture::TextureResource::Array(resource)
        | texture::TextureResource::Atlas { resource, .. } => &resource.texture,
    };
    Ok((texture.as_raw().width(), texture.as_raw().height()))
}

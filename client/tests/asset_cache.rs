//! Ownership regression: cache entries must not keep GPU resources alive.
use client::rendering::{AssetCache, GpuContext, SceneRenderer};
use velum::backend::texture;

#[test]
#[ignore = "requires a graphics adapter"]
fn render_data_owns_resources_independently_of_cache_and_renderer() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = GpuContext::from_device(device, queue, 32, 32, wgpu::TextureFormat::Rgba8UnormSrgb)
        .unwrap();
    let mut data = velum::RenderData::new();
    let mut cache = AssetCache::new(&gpu, &mut data).unwrap();
    let renderer = SceneRenderer::new(&gpu, &mut cache, &mut data).unwrap();
    let path = std::env::temp_dir().join(format!("alpha-cache-{}.png", std::process::id()));
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(&path)
        .unwrap();
    let id = client::rendering::texture::load_cached(&mut cache, &mut data, &path).unwrap();
    assert_eq!(id.context_id(), gpu.context_id());
    assert_eq!(client::rendering::texture::size(id).unwrap(), (1, 1));
    let texture = texture::get(id).unwrap();
    let texture::TextureResource::Separate(resource) = texture.as_ref() else {
        panic!("expected a separate texture");
    };
    assert_eq!(
        resource.texture.as_raw().format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
    drop(texture);
    assert_eq!(
        id,
        client::rendering::texture::load_cached(&mut cache, &mut data, &path).unwrap()
    );
    assert!(cache.remove_texture(&mut data, id));
    assert!(texture::get(id).is_none());
    let id = client::rendering::texture::load_cached(&mut cache, &mut data, &path).unwrap();
    std::fs::remove_file(path).unwrap();

    drop(renderer);
    drop(cache);
    assert!(texture::get(id).is_some());
    drop(data);
    assert!(texture::get(id).is_none());
    assert!(gpu.take_error().is_none());
}

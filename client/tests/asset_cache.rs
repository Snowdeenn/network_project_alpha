//! Ownership regression: cache entries must not keep GPU resources alive.
use client::rendering::{AssetCache, GpuContext, SceneRenderer};
use velum::backend::context;

#[test]
#[ignore = "requires a graphics adapter"]
fn render_data_owns_resources_independently_of_cache_and_renderer() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    // The pinned Velum version still registers textures in its default context.
    context::init_from(device.clone(), queue.clone()).unwrap();
    let gpu = GpuContext::from_device(device, queue, 32, 32, wgpu::TextureFormat::Rgba8UnormSrgb)
        .unwrap();
    let mut data = velum::RenderData::new();
    let mut cache = AssetCache::new(&gpu, &mut data).unwrap();
    let renderer = SceneRenderer::new(&gpu, &mut cache, &mut data).unwrap();
    let path = std::env::temp_dir().join(format!("alpha-cache-{}.png", std::process::id()));
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save(&path)
        .unwrap();
    let id = cache.load_texture(&gpu, &mut data, &path).unwrap();
    assert_eq!(id, cache.load_texture(&gpu, &mut data, &path).unwrap());
    assert!(cache.remove_texture(&mut data, id));
    assert!(
        context::texture_storage(None)
            .read()
            .unwrap()
            .get(id)
            .is_none()
    );
    let id = cache.load_texture(&gpu, &mut data, &path).unwrap();
    std::fs::remove_file(path).unwrap();

    drop(renderer);
    drop(cache);
    assert!(
        context::texture_storage(None)
            .read()
            .unwrap()
            .get(id)
            .is_some()
    );
    drop(data);
    assert!(
        context::texture_storage(None)
            .read()
            .unwrap()
            .get(id)
            .is_none()
    );
    assert!(gpu.take_error().is_none());
}

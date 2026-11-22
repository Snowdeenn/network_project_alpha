//! Native Velum resource lifetime checks; no Alpha storage or Prism projections.
use client::rendering::{AssetCache, BlendMode, GpuContext, Primitive, SceneFrame, SceneRenderer};
use velum::backend::{
    self as gpu,
    buffer::{self, BufferKind},
    geometry::BufferRegion,
    sampler,
    target::{self, RenderTargetDescriptor, TargetSize},
    texture,
};
fn context() -> GpuContext {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    GpuContext::from_device(device, queue, 32, 32, wgpu::TextureFormat::Rgba8UnormSrgb).unwrap()
}
fn destination(context: &GpuContext, size: u64) -> wgpu::Buffer {
    context.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}
fn read(context: &GpuContext, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    context
        .device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let value = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    value
}
#[test]
#[ignore = "requires a graphics adapter"]
fn native_buffer_leases_keep_allocations_live_until_submission() {
    struct CopyBuffer;
    impl BufferKind for CopyBuffer {
        const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::COPY_SRC;
    }
    let context = context();
    let ctx = Some(context.velum_context_id());
    let owner = buffer::allocate_in::<u8, CopyBuffer>(ctx, 16)
        .unwrap()
        .share();
    let id = owner.id();
    buffer::try_write_buffer(&id, 0, &[7u8; 16]).unwrap();
    let region = BufferRegion::from_shared(owner.clone()).unwrap();
    let another_object = region.clone();
    let (raw, range) = region.raw_region();
    let output = destination(&context, 16);
    let mut encoder = context.create_encoder("native buffer copy");
    encoder.copy_buffer_to_buffer(raw, range.start, &output, 0, 16);
    drop(owner);
    drop(another_object);
    assert!(id.try_raw_region().is_ok());
    context.submit(encoder);
    drop(region);
    assert!(id.try_raw_region().is_err());
    assert_eq!(read(&context, &output), [7u8; 16]);
    assert!(context.take_error().is_none());
}
#[test]
#[ignore = "requires a graphics adapter"]
fn native_texture_and_target_render_share_and_release_without_alpha_storage() {
    let context = context();
    let ctx = Some(context.velum_context_id());
    let mut data = velum::RenderData::new();
    let mut assets = AssetCache::new(&context, &mut data).unwrap();
    let mut renderer = SceneRenderer::new(&context, &mut assets, &mut data).unwrap();
    let sampler = sampler::pixel_art_in(ctx, "test sampler");
    let raw = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("shared native texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    context.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &raw,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 0, 0, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        raw.size(),
    );
    let owner = texture::try_register_in(ctx, texture::Texture::from_raw(raw), &sampler)
        .unwrap()
        .share();
    let id = owner.id();
    let another_object = owner.clone();
    assert_eq!(
        texture::get(id).unwrap().as_raw(),
        texture::get(id).unwrap().as_raw()
    );
    let output_owner = target::try_register_in(
        ctx,
        RenderTargetDescriptor {
            label: "output",
            size: TargetSize::Fixed {
                width: 4,
                height: 2,
            },
            format: context.surface_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            sample_count: 1,
        },
    )
    .unwrap();
    let output_id = output_owner.id();
    let output = target::get(output_id).unwrap();
    let mut scene = SceneFrame::new();
    for x in [0.0, 2.0] {
        scene.push_hud(Primitive::Texture {
            flash: false,
            id,
            pos: [x, 0.0],
            size: [2.0; 2],
            rotation: 0.0,
            uv: None,
            tint: [1.0; 4],
            blend: BlendMode::Alpha,
            layer: 0,
        });
    }
    let report = renderer
        .render_to_target(&context, &mut assets, &mut data, scene, output_id)
        .unwrap();
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let destination = destination(&context, 512);
    let mut encoder = context.create_encoder("native target copy");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: output.texture().as_raw(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &destination,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(2),
            },
        },
        output.texture().as_raw().size(),
    );
    drop(owner);
    assert!(texture::get(id).is_some());
    drop(another_object);
    assert!(texture::get(id).is_none());
    drop(output_owner);
    assert!(target::get(output_id).is_none());
    context.submit(encoder);
    for row in read(&context, &destination).chunks_exact(256) {
        for pixel in row[..16].chunks_exact(4) {
            assert_eq!(pixel, [255, 0, 0, 255]);
        }
    }
    assert!(context.take_error().is_none());
}
#[test]
#[ignore = "requires a graphics adapter"]
fn native_targets_resize_by_context_preserve_snapshots_and_release_by_owner() {
    let context = context();
    let ctx = Some(context.velum_context_id());
    let descriptor = RenderTargetDescriptor {
        label: "window",
        size: TargetSize::Window {
            width: 8,
            height: 8,
        },
        format: context.surface_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        sample_count: 1,
    };
    let owner = target::try_register_in(ctx, descriptor).unwrap();
    let id = owner.id();
    let old = target::get(id).unwrap();
    let old_view = old.texture().view().clone();
    target::resize_window_targets_in(ctx, [0, 0]);
    assert_eq!(target::get(id).unwrap().revision(), 0);
    target::resize_window_targets_in(ctx, [16, 4]);
    let new = target::get(id).unwrap();
    assert_eq!(new.size(), [16, 4]);
    assert_eq!(new.revision(), 1);
    assert_eq!(old.size(), [8, 8]);
    assert_ne!(new.texture().view(), &old_view);
    let mut invalid = descriptor;
    invalid.size = TargetSize::Fixed {
        width: 0,
        height: 4,
    };
    assert!(target::try_register_in(ctx, invalid).is_err());
    assert_eq!(target::get(id).unwrap().size(), [16, 4]);
    drop(owner);
    assert!(target::get(id).is_none());
    assert!(context.take_error().is_none());
    gpu::discard();
}

#[test]
#[ignore = "requires a graphics adapter"]
fn native_texture_registration_returns_gpu_validation_errors() {
    let local = context();
    let foreign = context();
    let sampler = sampler::pixel_art_in(Some(foreign.velum_context_id()), "foreign sampler");
    let raw = local.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("validation texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let value = texture::Texture::from_raw(raw);
    assert!(
        texture::try_register_in(Some(local.velum_context_id()), value.clone(), &sampler).is_err()
    );
    let local_sampler = sampler::pixel_art_in(Some(local.velum_context_id()), "local sampler");
    assert!(
        texture::try_register_in(Some(local.velum_context_id()), value, &local_sampler).is_err()
    );
    assert!(
        local.take_error().is_none(),
        "validation must be returned through the API"
    );
}

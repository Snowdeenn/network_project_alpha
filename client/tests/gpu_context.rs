//! Explicit GPU checks: cargo test -p client --features velum --test gpu_context -- --ignored
use client::rendering::{GpuContext, GpuError};

fn context() -> GpuContext {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("graphics adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    GpuContext::from_device(device, queue, 64, 64, wgpu::TextureFormat::Rgba8UnormSrgb).unwrap()
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn resize_preserves_gpu_and_format_and_rejects_invalid_dimensions() {
    let mut context = context();
    let device = context.device();
    let queue = context.queue();
    let format = context.surface_format();
    context.resize(0, 0).unwrap();
    assert_eq!((context.size.width, context.size.height), (0, 0));
    assert!(!context.is_ready());
    context.resize(96, 32).unwrap();
    assert_eq!((context.size.width, context.size.height), (96, 32));
    assert_eq!(context.surface_format(), format);
    assert_eq!(context.device(), device);
    assert_eq!(context.queue(), queue);
    let limit = device.limits().max_texture_dimension_2d;
    assert!(matches!(
        context.resize(limit + 1, 32),
        Err(GpuError::InvalidSurfaceSize { .. })
    ));
    assert_eq!((context.size.width, context.size.height), (96, 32));
    assert!(context.take_error().is_none());
}

#[cfg(feature = "velum")]
#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn velum_shares_the_gpu_and_unregisters_before_recreation() {
    let context = context();
    let device = context.device();
    let queue = context.queue();
    let id = context.context_id();
    {
        let registered = velum::backend::context::try_context(id).unwrap();
        assert_eq!(registered.device, device);
        assert_eq!(registered.queue, queue);
    }
    drop(context);
    assert!(velum::backend::context::try_context(id).is_none());
    let recreated = GpuContext::from_device(
        device.clone(),
        queue.clone(),
        32,
        32,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    )
    .unwrap();
    assert_ne!(recreated.context_id(), id);
    let registered = velum::backend::context::try_context(recreated.context_id()).unwrap();
    assert_eq!(registered.device, device);
    assert_eq!(registered.queue, queue);
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn uncaptured_validation_errors_reach_the_application() {
    let context = context();
    // MAP_READ may only be combined with COPY_DST on this default device.
    let _invalid = context.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("intentional validation failure"),
        size: 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    context.device().poll(wgpu::PollType::Poll).unwrap();
    let error = context.take_error().expect("GPU validation callback");
    assert!(matches!(
        error,
        GpuError::Uncaptured(wgpu::Error::Validation { .. })
    ));
    assert!(!error.is_fatal());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn device_loss_is_reported_as_fatal() {
    let context = context();
    context.device().destroy();
    context.device().poll(wgpu::PollType::Poll).unwrap();
    let error = context.take_error().expect("device-lost callback");
    assert!(matches!(
        error,
        GpuError::DeviceLost {
            reason: wgpu::DeviceLostReason::Destroyed,
            ..
        }
    ));
    assert!(error.is_fatal());
}

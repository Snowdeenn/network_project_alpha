//! Creates a short-lived X11 window to exercise real surface presentation.
#![cfg(target_os = "linux")]
use client::rendering::{AssetCache, GpuContext, SceneFrame, SceneRenderer};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::x11::EventLoopBuilderExtX11,
    window::{Window, WindowId},
};

fn draw(
    context: &GpuContext,
    renderer: &mut SceneRenderer,
    assets: &mut AssetCache,
    data: &mut velum::RenderData,
) {
    for _ in 0..16 {
        if let Some(frame) = renderer
            .render(context, assets, data, SceneFrame::new())
            .unwrap()
        {
            assert_eq!(
                frame.surface_texture.texture.format(),
                context.surface_format()
            );
            assert_eq!(frame.size, (context.size.width, context.size.height));
            assert_eq!(
                (
                    frame.surface_texture.texture.width(),
                    frame.surface_texture.texture.height()
                ),
                (context.size.width, context.size.height)
            );
            frame.present(context);
            context
                .device()
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .unwrap();
            assert!(context.take_error().is_none());
            return;
        }
    }
    panic!("surface did not produce a frame");
}

#[derive(Default)]
struct Probe {
    completed: bool,
}

impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Alpha GPU context test")
                        .with_inner_size(winit::dpi::PhysicalSize::new(64, 64)),
                )
                .unwrap(),
        );
        let mut context = pollster::block_on(GpuContext::new(window.clone())).unwrap();
        let device = context.device();
        let queue = context.queue();
        let format = context.surface_format();
        let mut data = velum::RenderData::new();
        let mut assets = AssetCache::new(&context, &mut data).unwrap();
        let mut renderer = SceneRenderer::new(&context, &mut assets, &mut data).unwrap();
        draw(&context, &mut renderer, &mut assets, &mut data);

        context.resize(0, 0).unwrap();
        assert!(!context.is_ready());
        assert!(
            renderer
                .render(&context, &mut assets, &mut data, SceneFrame::new())
                .unwrap()
                .is_none()
        );
        for [width, height] in [[96, 32], [32, 96], [64, 64]] {
            context.resize(width, height).unwrap();
            assert!(context.is_ready());
            assert_eq!(context.surface_format(), format);
            assert_eq!(context.device(), device);
            assert_eq!(context.queue(), queue);
            draw(&context, &mut renderer, &mut assets, &mut data);
        }
        context.reconfigure();
        draw(&context, &mut renderer, &mut assets, &mut data);
        drop(renderer);
        drop(assets);
        drop(data);
        #[cfg(feature = "velum")]
        let old_id = context.velum_context_id();
        drop(context);
        #[cfg(feature = "velum")]
        assert!(velum::backend::context::try_context(Some(old_id)).is_none());

        let context = pollster::block_on(GpuContext::new(window)).unwrap();
        assert_eq!(context.surface_format(), format);
        let mut data = velum::RenderData::new();
        let mut assets = AssetCache::new(&context, &mut data).unwrap();
        let mut renderer = SceneRenderer::new(&context, &mut assets, &mut data).unwrap();
        draw(&context, &mut renderer, &mut assets, &mut data);
        self.completed = true;
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: winit::event::WindowEvent) {}
}

#[test]
#[ignore = "requires a graphics adapter and an X11 display; opens a temporary window"]
fn surface_presents_after_resize_minimization_and_recreation() {
    let event_loop = EventLoop::builder()
        .with_x11()
        .with_any_thread(true)
        .build()
        .unwrap();
    let mut probe = Probe::default();
    event_loop.run_app(&mut probe).unwrap();
    assert!(probe.completed);
}

//! Creates a short-lived desktop window to exercise real surface presentation.
#![cfg(any(target_os = "linux", target_os = "windows"))]
use client::rendering::{AssetCache, GpuContext, SceneFrame, SceneRenderer};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn draw(
    context: &GpuContext,
    renderer: &mut SceneRenderer,
    assets: &mut AssetCache,
    data: &mut velum::RenderData,
) {
    for _ in 0..16 {
        let mut scene = SceneFrame::new();
        scene.push_world(client::rendering::Primitive::Shape {
            shape: velum::geometry::Shape::Quad {
                pos: [0.; 2],
                size: [8.; 2],
                rotation: 0.,
                color: [1.; 4],
                uv: None,
            },
            blend: client::rendering::BlendMode::Alpha,
            layer: 0,
        });
        scene.push_hud(client::rendering::Primitive::Text {
            content: "Velum".into(),
            pos: [0., 0.],
            size: 12.,
            color: [1.; 4],
            layer: 0,
        });
        if let Some(mut frame) = renderer.render(context, assets, data, scene).unwrap() {
            assert_eq!(
                frame.render_parts().1.texture().format(),
                context.surface_format()
            );

            assert_eq!(
                frame.size_in_pixels(),
                [context.size.width, context.size.height]
            );
            frame.finish(false);
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
    panic!("surface did not produce a frame at {:?}", context.size);
}

#[derive(Default)]
struct Probe {
    completed: bool,
    window: Option<Arc<Window>>,
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
        window.request_redraw();
        self.window = Some(window);
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: WindowId,
        event: winit::event::WindowEvent,
    ) {
        if !matches!(event, winit::event::WindowEvent::RedrawRequested) || self.completed {
            return;
        }
        let window = self.window.as_ref().unwrap().clone();
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
            let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(width, height));
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
        let old_id = context.context_id();
        context.replace_window(window).unwrap();
        #[cfg(feature = "velum")]
        assert_eq!(old_id, context.context_id());
        assert_eq!(context.surface_format(), format);
        let mut data = velum::RenderData::new();
        let mut assets = AssetCache::new(&context, &mut data).unwrap();
        let mut renderer = SceneRenderer::new(&context, &mut assets, &mut data).unwrap();
        draw(&context, &mut renderer, &mut assets, &mut data);
        self.completed = true;
        event_loop.exit();
    }
}

#[test]
#[ignore = "requires a desktop graphics adapter and window system"]
fn surface_presents_after_resize_minimization_and_recreation() {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11().with_any_thread(true);
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    let event_loop = builder.build().unwrap();
    let mut probe = Probe::default();
    event_loop.run_app(&mut probe).unwrap();
    assert!(probe.completed);
}

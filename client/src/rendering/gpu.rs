//! Native Velum context and surface ownership for the Alpha event loop.
use std::sync::{Arc, mpsc};

#[derive(Debug)]
pub enum GpuError {
    Initialization(velum::backend::renderer::RendererInitError),
    InvalidSurfaceSize {
        width: u32,
        height: u32,
        limit: u32,
    },
    Uncaptured(wgpu::Error),
    DeviceLost {
        reason: wgpu::DeviceLostReason,
        message: String,
    },
}

impl GpuError {
    pub fn is_fatal(&self) -> bool {
        !matches!(self, Self::Uncaptured(wgpu::Error::Validation { .. }))
    }
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialization(error) => write!(f, "GPU initialization: {error}"),
            Self::InvalidSurfaceSize {
                width,
                height,
                limit,
            } => {
                write!(
                    f,
                    "invalid surface size {width}x{height} (device limit {limit})"
                )
            }
            Self::Uncaptured(error) => write!(f, "uncaptured GPU error: {error}"),
            Self::DeviceLost { reason, message } => {
                write!(f, "GPU device lost ({reason:?}): {message}")
            }
        }
    }
}

impl std::error::Error for GpuError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Initialization(error) => Some(error),
            Self::Uncaptured(error) => Some(error),
            _ => None,
        }
    }
}

/// Owns one native Velum context and its optional native window renderer.
pub struct GpuContext {
    velum: velum::OwnedResource<velum::backend::context::ContextId>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    presentation: Option<velum::backend::renderer::Renderer>,
    window: Option<Arc<winit::window::Window>>,
    pub size: winit::dpi::PhysicalSize<u32>,
    format: wgpu::TextureFormat,
    errors: mpsc::Receiver<GpuError>,
}
impl GpuContext {
    pub async fn new(window: Arc<winit::window::Window>) -> Result<Self, GpuError> {
        let (renderer, owner) = velum::backend::renderer::Renderer::new_registered(window.clone())
            .map_err(GpuError::Initialization)?;
        let size = window.inner_size();
        let format = renderer.conf.format;
        Ok(Self::attach(
            owner,
            Some(renderer),
            Some(window),
            size,
            format,
        ))
    }
    pub fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GpuError> {
        let limit = device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > limit || height > limit {
            return Err(GpuError::InvalidSurfaceSize {
                width,
                height,
                limit,
            });
        }
        let owner = velum::backend::context::register(device, queue);
        Ok(Self::attach(
            owner,
            None,
            None,
            winit::dpi::PhysicalSize::new(width, height),
            format,
        ))
    }
    fn attach(
        velum: velum::OwnedResource<velum::backend::context::ContextId>,
        presentation: Option<velum::backend::renderer::Renderer>,
        window: Option<Arc<winit::window::Window>>,
        size: winit::dpi::PhysicalSize<u32>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let gpu = velum::backend::context::context(Some(velum.id()));
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        let (sender, errors) = mpsc::channel();
        let lost_sender = sender.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            let _ = sender.send(GpuError::Uncaptured(error));
        }));
        device.set_device_lost_callback(move |reason, message| {
            let _ = lost_sender.send(GpuError::DeviceLost { reason, message });
        });
        Self {
            velum,
            device,
            queue,
            presentation,
            window,
            size,
            format,
            errors,
        }
    }
    pub fn velum_context_id(&self) -> velum::backend::context::ContextId {
        self.velum.id()
    }
    pub fn take_error(&self) -> Option<GpuError> {
        self.errors.try_recv().ok()
    }
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.format
    }
    pub fn is_ready(&self) -> bool {
        self.presentation.is_some() && self.size.width > 0 && self.size.height > 0
    }
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), GpuError> {
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(GpuError::InvalidSurfaceSize {
                width,
                height,
                limit,
            });
        }
        self.size = winit::dpi::PhysicalSize::new(width, height);
        if width > 0 && height > 0 {
            if let Some(renderer) = &mut self.presentation {
                renderer.conf.width = width;
                renderer.conf.height = height;
                renderer.surface.configure(&self.device, &renderer.conf);
            }
            velum::backend::target::resize_window_targets_in(
                Some(self.velum.id()),
                [width, height],
            );
        }
        Ok(())
    }
    pub fn reconfigure(&self) {
        if self.is_ready()
            && let Some(renderer) = &self.presentation
        {
            renderer.surface.configure(&self.device, &renderer.conf);
        }
    }
    pub fn current_texture(&self) -> wgpu::CurrentSurfaceTexture {
        self.presentation
            .as_ref()
            .map_or(wgpu::CurrentSurfaceTexture::Validation, |renderer| {
                renderer.surface.get_current_texture()
            })
    }
    pub fn create_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }
    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit([encoder.finish()]);
    }
    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        self.queue.present(frame);
    }
}

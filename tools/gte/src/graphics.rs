use std::sync::Arc;
use tracing::error;
use winit::window::Window;
use wgpu::{CreateSurfaceError, Features, Limits, MemoryHints, Surface};
use wgpu::Backend::Gl;
use gte_core::emulator::{HEIGHT, WIDTH};

pub struct GraphicsContext {
    pub _instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>) -> Self {
        // force webgl on web
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            backend_options: Default::default(),
        });

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());


        #[cfg(target_arch = "wasm32")]
        loop {
            use gloo_timers::future::TimeoutFuture;
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                break;
            }
            // Wait one frame (roughly 16ms) before checking again
            TimeoutFuture::new(16).await;
        }

        let surface = match instance.create_surface(window.clone()) {
            Ok(ok) => {ok}
            Err(e) => {panic!("Failed to create surface: {:?}", e)}
        };

        let adapter = match instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }).await {
            Some(adapter) => adapter,
            None => {
                error!("failed to find adapter, forcing fallback");
                instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::None,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                }).await.expect("Failed to find fallback adapter")
            }
        };


        let features = Features::default();
        
        let mut limits = Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_1d = 8192;
        limits.max_texture_dimension_2d = 8192;

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: features,
            required_limits: limits,
            memory_hints: MemoryHints::default(),
        }, None).await.unwrap();

        let swapchain_capabilities = surface.get_capabilities(&adapter);

        let swapchain_format = swapchain_capabilities
            .formats.iter()
            .find(|&&fmt| fmt == wgpu::TextureFormat::Rgba8Unorm || fmt == wgpu::TextureFormat::Bgra8Unorm)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width: window.inner_size().width.max(128),
            height: window.inner_size().height.max(128),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        Self {
            _instance: instance,
            device,
            queue,
            surface_config,
            surface,
        }
    }
}
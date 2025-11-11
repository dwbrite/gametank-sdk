use std::sync::{mpsc, Arc};
use std::time::Instant;
use egui::{epaint, Color32, TextureHandle, TextureOptions, Ui};
use egui::UiKind::CentralPanel;
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
use winit::window::{Window, WindowAttributes, WindowId};

use egui_wgpu::{wgpu as wgpu, ScreenDescriptor};
use egui_wgpu::wgpu::{Limits, MemoryHints};
use tracing::{debug, info, warn};
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use web_sys::{Document, HtmlCanvasElement, HtmlElement};
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, KeyEvent, StartCause, WindowEvent};
use crate::app_initialized::AppInitialized;
use crate::app_ui::gametankboy::GameTankBoyUI;
use gte_core::color_map::COLOR_MAP;
use crate::egui_renderer::EguiRenderer;
use gte_core::emulator::{Emulator, HEIGHT, WIDTH};
use crate::app_delegation::{InstantClock};
use crate::graphics::GraphicsContext;



pub struct App {
    pub emulator: Option<Emulator<InstantClock>>,
    pub gc: Option<GraphicsContext>,
    pub window: Option<Arc<Window>>,
    pub egui_renderer: Option<EguiRenderer>,

    pub app_initialized: Option<AppInitialized>,

    pub gc_tx: mpsc::Sender<GraphicsContext>,
    pub gc_rx: mpsc::Receiver<GraphicsContext>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let clock = InstantClock {
            instant: Instant::now(),
        };

        Self {
            emulator: Some(Emulator::init(clock, 48000.0)),
            gc: None,
            window: None,
            egui_renderer: None,
            gc_tx: tx,
            gc_rx: rx,
            app_initialized: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn get_canvas(document: &Document) -> Option<HtmlCanvasElement> {
        // First, try to get the canvas from the light DOM.
        if let Some(elem) = document.get_element_by_id("gt-canvas") {
            return elem.dyn_into::<HtmlCanvasElement>().ok();
        }

        // If not found, try to locate the canvas in a shadow DOM.
        if let Some(shadow_host) = document.get_element_by_id("shadow-host") {
            if let Some(shadow_root) = shadow_host
                .dyn_ref::<HtmlElement>()
                .and_then(|host| host.shadow_root())
            {
                if let Ok(Some(canvas_elem)) = shadow_root.query_selector("#gt-canvas") {
                    return canvas_elem.dyn_into::<HtmlCanvasElement>().ok();
                }
            }
        }

        None
    }

    fn init_window(&mut self, event_loop: &ActiveEventLoop) {
        info!("initializing...");
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_title("GameTank: The Emulator!")
            .with_inner_size(LogicalSize::new((128*4), (128*4)+24))
            .with_min_inner_size(LogicalSize::new(WIDTH, HEIGHT));
        
        

        #[cfg(target_arch = "wasm32")] {
            window_attributes = window_attributes.with_inner_size(LogicalSize::new(128, 128));
            use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
            use web_sys::{HtmlCanvasElement, HtmlElement};
            use wasm_bindgen::JsCast;

            let window = web_sys::window().expect("should have a Window");
            let document = window.document().expect("should have a Document");

            let canvas = Self::get_canvas(&document).expect("should have a canvas element");

            let canvas: HtmlCanvasElement = canvas.dyn_into::<HtmlCanvasElement>().expect("failed to transmute canvas element");
            warn!("found canvas: ({}, {})", canvas.width(), canvas.height());
            window_attributes = window_attributes.with_canvas(Some(canvas));
        }


        let window = Arc::new(event_loop.create_window(window_attributes).expect("failed to create window"));
        self.window = Some(window.clone());

        let window_clone = window.clone();
        let tx_clone = self.gc_tx.clone();
        crate::spawn(async move {
            let gc = GraphicsContext::new(window_clone).await;
            tx_clone.send(gc).expect("couldn't send");
        });

        self.try_graphics_context();

        info!("initialized");
    }

    fn try_graphics_context(&mut self) {
        if let Some(window) = self.window.as_ref() {
            if let Ok(gc) = self.gc_rx.try_recv() {
                let device = &gc.device;

                let fmt = gc.surface.get_current_texture().expect("ugh").texture.format();

                self.egui_renderer = Some(EguiRenderer::new(device, fmt, None, 1, &window));
                // let color_image = self.framebuffer_to_color_image(&self.emulator.cpu_bus.read_full_framebuffer());
                self.gc = Some(gc);
                info!("adapter has been set up");
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.init_window(event_loop);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.gc.is_none() {
            self.try_graphics_context();
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if self.gc.is_none() {
            return;
        }

        if self.gc.is_some() && self.egui_renderer.is_some() && self.window.is_some() && self.emulator.is_some() {
            warn!("initialized app");
            let app_init = AppInitialized::from(&mut *self);
            app_init.window.request_redraw();
            self.app_initialized = Some(app_init);
        }
    }
}

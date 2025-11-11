#![allow(clippy::disallowed_methods, clippy::single_match)]
#![allow(dead_code, unused_variables, unused_imports, internal_features)]

mod helpers;
mod app_uninit;
mod egui_renderer;
mod graphics;
mod app_ui;
pub mod app_initialized;
mod app_delegation;
mod audio;

use app_delegation::DelegatedApp::Uninitialized;
use std::cmp::PartialEq;
use tracing::{error, info, warn, Level};
use winit::event_loop::EventLoop;

use winit::event_loop::ControlFlow;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;

use tracing_subscriber::util::SubscriberInitExt;

#[cfg(target_arch = "wasm32")]
use web_sys::{window, HtmlCanvasElement};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
use std::future::Future;

#[cfg(target_arch = "wasm32")]
use web_sys::Event;
use crate::app_uninit::App;

fn setup_logging() {
    #[cfg(target_arch = "wasm32")]
    {
        use tracing_wasm::{WASMLayer, WASMLayerConfigBuilder};
        use tracing_subscriber::layer::SubscriberExt;

        // Set up the WASM layer for tracing logs
        let wlconfig = WASMLayerConfigBuilder::new()
            .set_max_level(Level::WARN).build();

        let wasm_layer = WASMLayer::new(wlconfig);
        // Configure the subscriber with the WASM layer
        tracing_subscriber::registry()
            .with(wasm_layer)
            .init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tracing_subscriber::fmt()
            .with_max_level(Level::WARN)
            .compact()
            .finish()
            .init();
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub fn wasm_main() {
    use std::panic;
    use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};

    panic::set_hook(Box::new(|panic_info| {
        // Log the panic info to console (using the default hook for formatting)
        console_error_panic_hook::hook(panic_info);
        // Dispatch a custom event to notify JS of the panic.
        if let Some(window) = web_sys::window() {
            let event = Event::new("wasm-panic").unwrap();
            window.dispatch_event(&event).unwrap();
        }
    }));

    setup_logging();
    info!("console logger started.");

    let event_loop = EventLoop::<()>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let app = Uninitialized(App::new());

    let _ = event_loop.spawn_app(app);
}

pub fn main() {
    // welcome to the main function!
    // If you want to see how the emulator works, the "main" modules are app_initialized and emulator.

    // app_delegation and app_unitinitailized are used for initializing the app,
    // namely grabbing winit/egui/wgpu resources.



    #[cfg(not(target_arch = "wasm32"))] {
        setup_logging();
        info!("stdout logger started");

        let event_loop = EventLoop::<()>::with_user_event().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        use thread_priority::*;
        // if it didn't work, oh well
        let _ = set_current_thread_priority(ThreadPriority::Max);

        let mut app = Uninitialized(App::new());
        // TODO: app.emulator.as_mut().unwrap().play_state = Playing;

        let _ = event_loop.run_app(&mut app);
    }
}

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);
    #[cfg(not(target_arch = "wasm32"))]
    pollster::block_on(future)
}
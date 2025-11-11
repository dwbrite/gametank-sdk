use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::window::WindowId;
use crate::app_initialized::AppInitialized;
use crate::app_uninit::App;
use gte_core::emulator::TimeDaemon;

pub struct InstantClock {
    pub instant: Instant,
}

impl TimeDaemon for InstantClock {
    fn get_now_ms(&self) -> f64 {
        self.instant.elapsed().as_millis() as f64
    }
}

pub enum DelegatedApp {
    Uninitialized(App),
    Initialized(AppInitialized),
}

impl ApplicationHandler for DelegatedApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.new_events(event_loop, cause),
            DelegatedApp::Initialized(ref mut app) => app.new_events(event_loop, cause),
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.resumed(event_loop),
            DelegatedApp::Initialized(ref mut app) => app.resumed(event_loop),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.user_event(event_loop, event),
            DelegatedApp::Initialized(ref mut app) => app.user_event(event_loop, event),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.window_event(event_loop, window_id, event),
            DelegatedApp::Initialized(ref mut app) => app.window_event(event_loop, window_id, event),
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.device_event(event_loop, device_id, event),
            DelegatedApp::Initialized(ref mut app) => app.device_event(event_loop, device_id, event),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => {
                app.about_to_wait(event_loop);
                
                if let Some(app_initialized) = app.app_initialized.take() {
                    *self = DelegatedApp::Initialized(app_initialized);
                }
            }
            DelegatedApp::Initialized(ref mut app) => app.about_to_wait(event_loop),
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.suspended(event_loop),
            DelegatedApp::Initialized(ref mut app) => app.suspended(event_loop),
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.exiting(event_loop),
            DelegatedApp::Initialized(ref mut app) => app.exiting(event_loop),
        }
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            DelegatedApp::Uninitialized(ref mut app) => app.memory_warning(event_loop),
            DelegatedApp::Initialized(ref mut app) => app.memory_warning(event_loop),
        }
    }
}
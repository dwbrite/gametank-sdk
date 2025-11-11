use std::cell::{Cell, OnceCell};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use egui::{epaint, vec2, Align, Button, Color32, Frame, Id, LayerId, Layout, Pos2, Rect, ResizeDirection, ScrollArea, TextureOptions, Ui, UiBuilder, Vec2, ViewportCommand};
use egui_wgpu::ScreenDescriptor;
use klingt::Klingt;
use tracing::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use crate::app_ui::gametankboy::GameTankBoyUI;
use crate::app_ui::ram_inspector::MemoryInspector;
use crate::app_ui::vram_viewer::{VRAMViewer, VRAMViewerLayout};
use crate::app_uninit::App;
use gte_core::color_map::{COLOR_MAP, COLOR_MAP_PERCEPTUALLY_AUTOMAPPED, COLOR_MAP_WRONG};
use crate::egui_renderer::EguiRenderer;
use gte_core::emulator::{Emulator, HEIGHT, WIDTH};
use crate::graphics::GraphicsContext;
use crate::audio::GameTankAudio; // <--- added


pub struct AppInitialized {
    pub emulator: Emulator<InstantClock>,
    pub gc: GraphicsContext,
    pub window: Arc<Window>,
    pub egui_renderer: EguiRenderer,

    pub console_gui: GameTankBoyUI,
    pub vram_viewer: VRAMViewer,
    pub mem_inspector: MemoryInspector,

    pub input_bindings: HashMap<winit::keyboard::Key, InputCommand>,

    show_left_pane: bool,
    show_right_pane: bool,
    show_bottom_pane: bool,

    audio: Option<GameTankAudio>,
}

impl From<&mut App> for AppInitialized {
    fn from(app: &mut App) -> Self {
        let mut emulator = app.emulator.take().unwrap();
        let mut gc = app.gc.take().unwrap();
        let window = app.window.take().unwrap();
        let egui_renderer = app.egui_renderer.take().unwrap();
        let console_gui = GameTankBoyUI::init(egui_renderer.context(), Self::buffer_to_color_image(&emulator.cpu_bus.read_full_framebuffer()));
        let vram_viewer = VRAMViewer::new(VRAMViewerLayout::Pages, egui_renderer.context(), &mut emulator);

        gc.surface_config.width = window.inner_size().width;
        gc.surface_config.height = window.inner_size().height;
        gc.surface.configure(&gc.device, &gc.surface_config);

        let mut input_bindings : HashMap<keyboard::Key, gte_core::inputs::InputCommand> = HashMap::new();

        input_bindings.insert(keyboard::Key::Named(Enter), Controller1(ControllerButton::Start));
        input_bindings.insert(keyboard::Key::Named(ArrowUp), Controller1(ControllerButton::Up));
        input_bindings.insert(keyboard::Key::Named(ArrowDown), Controller1(ControllerButton::Down));
        input_bindings.insert(keyboard::Key::Named(ArrowLeft), Controller1(ControllerButton::Left));
        input_bindings.insert(keyboard::Key::Named(ArrowRight), Controller1(ControllerButton::Right));
        input_bindings.insert(keyboard::Key::Character(SmolStr::new("z")), Controller1(ControllerButton::A));
        input_bindings.insert(keyboard::Key::Character(SmolStr::new("x")), Controller1(ControllerButton::B));
        input_bindings.insert(keyboard::Key::Character(SmolStr::new("c")), Controller1(ControllerButton::C));

        if let Some(filename) = std::env::args().nth(1) {
            if let Ok(data) = std::fs::read(filename) {
                emulator.load_rom(&data);
                emulator.play_state = Playing;
            } else {
                error!("couldn't open provided file");
            }
        }

        // Create audio bridge if emulator already has audio_out (don't take or clone the ring endpoints)
        let audio_bridge = if emulator.audio_out.is_some() {
            Some(GameTankAudio::new())
        } else {
            None
        };

        Self {
            emulator,
            gc,
            window,
            egui_renderer,
            console_gui,
            vram_viewer,
            mem_inspector: MemoryInspector {},
            input_bindings,
            show_left_pane: false,
            show_right_pane: false,
            show_bottom_pane: false,
            audio: audio_bridge,
        }
    }
}

impl AppInitialized {
    fn handle_redraw(&mut self) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.gc.surface_config.width, self.gc.surface_config.height],
            pixels_per_point: self.window.scale_factor() as f32 * 1.0,
        };

        let surface_texture = self.gc
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.gc
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.egui_renderer.begin_frame(&self.window);
        let frame = egui::Frame {
            inner_margin: egui::Margin::same(0),
            outer_margin: egui::Margin::same(0),
            shadow: epaint::Shadow::default(),
            ..Default::default()
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            egui::TopBottomPanel::bottom("bottom_pane_2").resizable(false).show_separator_line(true).show_animated(self.egui_renderer.context(), self.show_bottom_pane, |ui| {
                ui.vertical(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.allocate_space(vec2(ui.available_width(), 0.0));
                        self.vram_viewer.draw(ui, &mut self.emulator);
                        ui.allocate_space(vec2(ui.available_width(), 0.0));
                    });
                });
            });

            egui::TopBottomPanel::bottom("bottom_pane_1").resizable(false).show_separator_line(true).show(self.egui_renderer.context(), |ui| {
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.show_left_pane, "show left panel");
                    ui.toggle_value(&mut self.show_bottom_pane, "show bottom panel");
                    ui.toggle_value(&mut self.show_right_pane, "show right panel");
                });
            });

            let mut left_size = 0.0;
            let mut right_size = 0.0;

            egui::SidePanel::left("left_pane").resizable(true).min_width(0.0).show_separator_line(true).frame(Frame {
                inner_margin: vec2(0.0, 0.0).into(),
                outer_margin: vec2(0.0, 0.0).into(),
                fill: Color32::from_gray(24),
                ..Default::default()
            }).show_animated(self.egui_renderer.context(), self.show_left_pane, |ui| {
                left_size = ui.available_width();

                if self.show_left_pane {
                    self.mem_inspector.draw(ui, &mut self.emulator);
                }
            });

            egui::SidePanel::right("right_pane").resizable(true).min_width(0.0).show_separator_line(true).frame(Frame {
                inner_margin: vec2(0.0, 0.0).into(),
                outer_margin: vec2(0.0, 0.0).into(),
                fill: Color32::from_gray(24),
                ..Default::default()
            }).show_animated(self.egui_renderer.context(), self.show_right_pane, |ui| {
                right_size = ui.available_width();

                if self.show_right_pane {
                    let sa = ScrollArea::both().enable_scrolling(true).min_scrolled_width(0.0).show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::RIGHT), |ui| {
                            Frame::default().show(ui, |ui| {
                                ui.set_min_width(24.0);
                                // ui.set_width(ui.available_width());
                                ui.set_height(ui.available_height());
                                ui.label("here's some gui shit");
                            })
                        });

                        ui.allocate_space(ui.available_size());
                    });
                }
            });
        }

        egui::CentralPanel::default().frame(frame).show(self.egui_renderer.context(), |ui| {
            // Set the minimum size for the center pane
            let center_min_size = egui::vec2(128.0, 128.0);
            ui.set_min_size(center_min_size);
            ui.horizontal_centered(|ui| {
                ui.set_height(ui.available_height());
                self.console_gui.draw(ui, &mut self.emulator);
            });
        });

        self.egui_renderer.end_frame_and_draw(
            &self.gc.device,
            &self.gc.queue,
            &mut encoder,
            &self.window,
            &surface_view,
            screen_descriptor,
        );

        self.gc.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }


    pub fn buffer_to_color_image(framebuffer: &[u8; 128*128]) -> egui::ColorImage {
        let mut pixels: Vec<u8> = Vec::with_capacity(128 * 128 * 4); // 4 channels per pixel (RGBA)

        for &index in framebuffer.iter() {
            let (r, g, b, a) = COLOR_MAP[index as usize];
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }

        egui::ColorImage::from_rgba_unmultiplied([128, 128], &pixels)
    }


    fn handle_resized(&mut self, width: u32, height: u32) {
        self.gc.surface_config.width = width;
        self.gc.surface_config.height = height;
        self.gc.surface.configure(&self.gc.device, &self.gc.surface_config);
    }
}

use std::cell::RefCell;
use std::collections::HashMap;
use gte_core::emulator::PlayState::{Paused, Playing};
use gte_core::inputs::{ControllerButton, InputCommand, KeyState};
use gte_core::inputs::InputCommand::Controller1;
use wasm_bindgen::prelude::*;
use winit::event::ElementState::Pressed;
use winit::keyboard;
use winit::keyboard::NamedKey::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Enter};
use winit::keyboard::SmolStr;
use crate::app_delegation::InstantClock;

// Use `thread_local!` to store per-thread global data in WASM
thread_local! {
    static ROM_DATA: RefCell<Option<Vec<u8>>> = RefCell::new(None);
    static SHOULD_SHUTDOWN: Cell<bool> = Cell::new(false);
    static EMULATOR_STOP: Cell<bool> = Cell::new(false);
}

// Function to update the ROM data from JavaScript
#[wasm_bindgen]
pub fn update_rom_data(data: &[u8]) {
    warn!("Loading new ROM into rust memory");
    ROM_DATA.with(|storage| {
        *storage.borrow_mut() = Some(data.to_vec());
    });
}

#[wasm_bindgen]
pub fn request_close() {
    warn!("Closing egui");
    SHOULD_SHUTDOWN.with(|flag| flag.set(true));
}

#[wasm_bindgen]
pub fn emulator_stop() {
    warn!("Stopping emulator");
    EMULATOR_STOP.with(|flag| flag.set(true));
}

impl AppInitialized {
    pub fn process_cycles(&mut self) {
        self.emulator.process_cycles(false);

        // If emulator created audio after initialization, create the bridge.
        if self.audio.is_none() && self.emulator.audio_out.is_some() {
            self.audio = Some(GameTankAudio::new());
        }

        // Drain whatever the emulator pushed into its own buffer and forward into our bridge.
        if let (Some(ref mut audio_out), Some(ref mut audio)) = (&mut self.emulator.audio_out, &mut self.audio) {
            while let Ok(buf) = audio_out.output_buffer.pop() {
                audio.push_buffer(buf);
            }
        }

        // Drive the audio bridge if present. It will pull from the bridge's internal consumer.
        if let Some(ref mut audio) = self.audio {
            audio.process_audio();
        }

        // previous manual draining of audio_out is removed because the bridge now owns the consumer
    }
}

impl ApplicationHandler for AppInitialized {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // nothing to do, probably?
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        self.process_cycles();
        self.egui_renderer.handle_input(&self.window, &event);

        // TODO: if playing and audio isn't init, init audio

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                self.window.request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let KeyEvent {  logical_key,   state,  .. } = event;
                if let Some(cmd) = self.input_bindings.get(&logical_key).copied() {
                    if let Some(ks) = self.emulator.input_state.get(&cmd) {
                        self.emulator.set_input_state(cmd, ks.update_state(state==Pressed))
                    } else {
                        self.emulator.set_input_state(cmd, KeyState::new(state==Pressed))
                    };
                }
            },
            WindowEvent::MouseInput { .. } => { self.emulator.wasm_init(); }
            WindowEvent::Touch(_) => { self.emulator.wasm_init(); }
            WindowEvent::DroppedFile(path) => {
                warn!("reading file from path...");
                // check if filename ends in .gtr and load file into slice
                let filename = path.file_name().unwrap().to_str().unwrap();
                if !filename.ends_with(".gtr") {
                    error!("not a valid gtr");
                    return
                }

                let mut file = File::open(&path).unwrap();
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).unwrap();

                self.emulator.load_rom(bytes.as_slice());
                warn!("successfully loaded {}", filename);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Check if a new ROM is waiting
        if let Some(data) = &ROM_DATA.take() {
            warn!("got rom data!");
            if !data.is_empty() {
                self.emulator.load_rom(data);
            }
            self.emulator.play_state = Playing;
        }

        if EMULATOR_STOP.with(|flag| flag.get()) {
            self.emulator.play_state = Paused;
        }

        if SHOULD_SHUTDOWN.with(|flag| flag.get()) {
            event_loop.exit();
        }

        self.process_cycles();
    }
}
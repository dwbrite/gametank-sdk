use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor, Read, Seek};
use egui::{include_image, vec2, Button, Color32, ColorImage, Context, Frame, ImageOptions, ImageSource, Rect, Shadow, SizeHint, Style, TextureHandle, TextureOptions, Ui, Vec2, Widget};
use egui::load::{SizedTexture, TextureLoadResult, TexturePoll};
use gte_core::emulator::Emulator;
use gte_core::emulator::PlayState::{Paused, Playing, WasmInit};
use image::{GenericImageView, ImageFormat};
use tracing::warn;
use crate::app_delegation::InstantClock;
use crate::egui_renderer::EguiRenderer;
use crate::graphics::GraphicsContext;

const MIN_GAME_SIZE: f32 = 128.0;

fn calculate_game_size(width: f32, height: f32, min_size: f32) -> f32 {
    let min_dimension = width.min(height);
    (min_dimension / min_size).floor() * min_size
}


pub struct GameTankBoyUI {
    desired_scale: Option<u8>,
    screen: Box<TextureHandle>,
    textures: HashMap<String, TextureHandle>,

    // a: [TextureHandle; 2],
    // b: [TextureHandle; 2],
    // c: [TextureHandle; 2],
    // start: TextureHandle,
    // up: TextureHandle,
    // down: TextureHandle,
    // left: TextureHandle,
    // right: TextureHandle,
    // power: [TextureHandle; 2],
    // reset: [TextureHandle; 2],
}

fn load_png_to_image(path: &str) -> ColorImage {
    // Load the image using the image crate
    let img = image::open(path).expect("Failed to load image");
    let rgb_image = img.to_rgba8();

    // Get the dimensions of the image
    let dimensions = img.dimensions();
    let size = [dimensions.0 as usize, dimensions.1 as usize];

    // Convert the image to egui::ColorImage
    let pixels = rgb_image.as_raw();

    ColorImage::from_rgba_unmultiplied(size, pixels)
}

fn load_png_bytes_to_image<R: BufRead + Seek>(bytes: R) -> ColorImage {
    let img = image::load(bytes, ImageFormat::Png).expect("failed to load image from bytes");

    let rgb_image = img.to_rgba8();

    // Get the dimensions of the image
    let dimensions = img.dimensions();
    let size = [dimensions.0 as usize, dimensions.1 as usize];

    // Convert the image to egui::ColorImage
    let pixels = rgb_image.as_raw();

    ColorImage::from_rgba_unmultiplied(size, pixels)
}

fn load_included_image(context: &Context, img: ImageSource) -> SizedTexture {
    match img.load(context, TextureOptions::NEAREST, SizeHint::Size(48, 48)).unwrap() {
        TexturePoll::Pending { .. } => {
            panic!("USE THIS PROPERLY.")
        }
        TexturePoll::Ready { texture } => {
            texture
        }
    }
}

impl GameTankBoyUI {
    pub fn init(context: &Context, color_image: ColorImage) -> Self {
        let options = TextureOptions::NEAREST;

        let game_texture = context.load_texture("game_texture", color_image, TextureOptions::NEAREST);

        let mut textures = HashMap::new();

        let power1 = context.load_texture("power_released", load_png_bytes_to_image(Cursor::new(include_bytes!("../assets/POWER1.png"))), options);
        let power2 = context.load_texture("power_released", load_png_bytes_to_image(Cursor::new(include_bytes!("../assets/POWER2.png"))), options);
        textures.insert("power_released".into(), power1);
        textures.insert("power_pressed".into(), power2);

        Self {
            desired_scale: Some(6),
            screen: Box::new(game_texture),
            textures
        }
    }

    pub fn update_screen(&mut self, color_image: ColorImage) {
        self.screen.set_partial([0, 0], color_image, TextureOptions::NEAREST);
    }

    pub fn draw(&mut self, ui: &mut Ui, emulator: &mut Emulator<InstantClock>) {
        // Convert framebuffer to ColorImage
        let color_image = {
            let framebuffer = emulator.cpu_bus.read_full_framebuffer();
            crate::app_initialized::AppInitialized::buffer_to_color_image(&framebuffer)
        };
        self.update_screen(color_image);

        let available_width = ui.available_width();
        let available_height = ui.available_height();
        let mut game_size = calculate_game_size(available_width, available_height, MIN_GAME_SIZE);
        let orig_scale = game_size / MIN_GAME_SIZE;

        // scale override, assuming there's enough space
        if let Some(scale) = self.desired_scale {
            if scale as f32 <= orig_scale {
                game_size = MIN_GAME_SIZE * scale as f32
            }
        }

        let sized_texture = egui::load::SizedTexture::new(self.screen.id(), vec2(game_size, game_size));

        let c = Color32::from_rgb(227, 190, 69);
        let frame = Frame {
            fill: c,
            ..Default::default()
        };

        frame.show(ui, |ui| {
            ui.vertical_centered(|ui| {
                // ui.style_mut().debug.debug_on_hover = true;
                ui.visuals_mut().widgets.active.bg_fill = c;

                let available_width = ui.available_width();
                // let available_height = ui.available_height();

                let margin_x = game_size * 0.2;
                let mut margin_y = game_size * 0.05;

                if available_height < game_size + margin_y * 2.0 {
                    margin_y = (available_height - game_size) / 2.0;
                }

                ui.set_width(available_width);
                ui.set_height(available_height);
                // game_rect.extend_with_x(64.0);

                // this is the screen:
                let frame_color = Color32::from_gray(8); // Light gray color for the frame
                let game_frame = Frame {
                    inner_margin: vec2(margin_x, margin_y).into(),
                    // rounding: Rounding::same(margin_y),
                    fill: frame_color,
                    outer_margin: vec2(0.0, 0.0).into(),
                    shadow:
                    Shadow {
                        offset: [0, 0],
                        blur: 2,
                        spread: 1,
                        color: Color32::from_rgb((c.r() as f32 * 0.4) as u8, (c.g() as f32 * 0.4) as u8, (c.b() as f32 * 0.2) as u8),
                    },
                    stroke: Default::default(),
                    corner_radius: Default::default(),
                };

                game_frame.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_width(game_size);
                        ui.set_height_range(0.0 ..= game_size);
                        ui.add(egui::Image::new(sized_texture));
                    })
                });

                let power = match emulator.play_state {
                    WasmInit => { self.textures.get("power_released").unwrap().clone() }
                    Paused => { self.textures.get("power_released").unwrap().clone() }
                    Playing => { self.textures.get("power_pressed").unwrap().clone() }
                };
                let btn_sized_texture = egui::load::SizedTexture::new(power.id(), Vec2::new(48.0, 48.0));
                let button = Button::image(egui::Image::new(btn_sized_texture)).frame(false);

                if button.ui(ui).clicked() {
                    match emulator.play_state {
                        WasmInit => { emulator.play_state = Playing; }
                        Paused => { emulator.play_state = Playing; }
                        Playing => { emulator.play_state = Paused; }
                    }
                }
            });
        });
    }
}
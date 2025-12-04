use std::time::Instant;
use egui::{vec2, Align, Context, Direction, Frame, Image, Layout, ScrollArea, Sense, TextureHandle, TextureOptions, Ui};
use egui::load::SizedTexture;
use egui::scroll_area::ScrollBarVisibility;
use egui::style::ScrollStyle;
use gte_core::emulator::Emulator;
use crate::app_delegation::InstantClock;

pub enum VRAMViewerLayout {
    Pages
}

const QUAD_SIZE: f32 = 128.0;

pub struct VRAMViewer {
    layout: VRAMViewerLayout,
    vram_quads: [TextureHandle; 32],
    framebuffers: [TextureHandle; 2],
    selected_page: usize,
}

impl VRAMViewer {
    pub fn new(layout: VRAMViewerLayout, context: &Context, emu: &mut Emulator<InstantClock>) -> Self {

        let mut quads = vec![];

        for (idx, bank) in emu.cpu_bus.vram_banks.iter().enumerate() {
            for chunk in bank.chunks_exact(128*128) {
                let color_image = {
                    let color_image: &[u8; 128 * 128] = chunk.try_into().expect("Chunk size mismatch");
                    crate::app_initialized::AppInitialized::buffer_to_color_image(color_image)
                };
                quads.push(context.load_texture(format!("vram{}", idx), color_image, TextureOptions::NEAREST));
            }
        }

        let framebuffers = emu.cpu_bus.framebuffers.iter_mut().enumerate().map(|f| {
            let (idx, fb) = f;
            let color_image = crate::app_initialized::AppInitialized::buffer_to_color_image(fb.get_mut());
            context.load_texture(format!("fb{}", idx), color_image, TextureOptions::NEAREST)
        }).collect::<Vec<_>>().try_into().ok().expect("Failed to convert framebuffer handles");

        let vram_quads = quads.try_into().ok().expect("Failed to convert quads to texture handles");

        Self {
            layout,
            vram_quads,
            framebuffers,
            selected_page: 0,
        }
    }

    pub fn draw(&mut self, ui: &mut Ui, emu: &mut Emulator<InstantClock>) {
        for (quad, was_written) in emu.cpu_bus.vram_quad_written.iter().enumerate() {
            if *was_written {
                let page = quad / 4;
                let page_quad = quad % 4;
                // read one quad
                let buffer = &emu.cpu_bus.vram_banks[page][page_quad*128*128..(page_quad+1)*128*128].try_into().expect("Chunk size mismatch");
                let color_image = crate::app_initialized::AppInitialized::buffer_to_color_image(buffer);
                self.vram_quads[quad].set_partial([0, 0], color_image, TextureOptions::NEAREST);
            }
        }

        let framebuffers = emu.cpu_bus.framebuffers.iter_mut().enumerate().map(|f| {
            let (idx, fb) = f;
            let color_image = crate::app_initialized::AppInitialized::buffer_to_color_image(fb.get_mut());
            self.framebuffers[idx].set_partial([0, 0], color_image, TextureOptions::NEAREST)
        }).collect::<Vec<_>>();

        match self.layout {
            VRAMViewerLayout::Pages => {
                let sa = ScrollArea::horizontal().enable_scrolling(true).drag_to_scroll(true).scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible);
                sa.show(ui,|ui| {
                    ui.set_height_range(0.0..=(256.0+32.0));
                    ui.set_width(1280.0);
                    let sa = ScrollArea::vertical().enable_scrolling(false).scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden);
                    sa.show(ui, |ui| {
                        // ui.set_height_range(256.0 + 32.0..=256.0 + 32.0);
                        ui.set_width(ui.available_width());
                        self.ui_pages(ui);
                        ui.allocate_space(vec2(0.0, ui.available_height()));
                    });
                });
            }
        }
    }

    fn ui_pages(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for page in 0..8 {
                let (size, separator) = if page == self.selected_page {
                    (QUAD_SIZE, 1.0)
                } else {
                    (QUAD_SIZE / 2.0, 0.0)
                };


                let q1 = SizedTexture::new(self.vram_quads[0 + 4*page].id(), vec2(size, size));
                let q2 = SizedTexture::new(self.vram_quads[1 + 4*page].id(), vec2(size, size));
                let q3 = SizedTexture::new(self.vram_quads[2 + 4*page].id(), vec2(size, size));
                let q4 = SizedTexture::new(self.vram_quads[3 + 4*page].id(), vec2(size, size));

                let response = ui.vertical(|ui| {
                    ui.label(format!("page {}", page));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, separator);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(separator, 0.0);
                            ui.image(q1);
                            ui.image(q2);
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(separator, 0.0);
                            ui.image(q3);
                            ui.image(q4);
                        });
                    });
                });

                if response.response.interact(Sense::click()).clicked() {
                    self.selected_page = page;
                }
            }
        });
    }
}
use egui::{Align, Color32, Label, Layout, RichText, Ui};
use egui_extras::Column;
use gte_core::emulator::Emulator;
use gte_core::gametank_bus::ByteDecorator;
use crate::app_delegation::InstantClock;

pub struct MemoryInspector {
    // memory: [ByteDecorator; 0x8000]
}

impl MemoryInspector {
    pub fn draw(&mut self, ui: &mut Ui, emulator: &mut Emulator<InstantClock>) {
        let bytes_per_line = 16;
        let total_lines = 0x8000 / bytes_per_line;


        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
        let tb = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto().at_least(40.0))  // Address column
            .columns(Column::auto().at_least(20.0), bytes_per_line)
            .resizable(false)
            // .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| { ui.label("Address"); });
                for i in 0..bytes_per_line {
                    header.col(|ui| { ui.label(format!("_{:X}", i)); });
                }
            })
            .body(|body| {
                body.rows(18.0, total_lines, |mut row| {
                    let row_idx = row.index();
                    row.col(|ui| {
                        let address = format!("{:04X}", row_idx * bytes_per_line);
                        ui.label(RichText::new(&address).color(Color32::WHITE).strong());
                    });

                    for column in 0..bytes_per_line {
                        let address = row_idx * bytes_per_line + column;
                        row.col(|ui| {
                            let (byte, color) = match emulator.cpu_bus.peek_byte_decorated(address as u16) {
                                ByteDecorator::ZeroPage(b) => { (b, Color32::from_rgb(0, 0, 0)) },
                                ByteDecorator::CpuStack(b) => { (b, Color32::from_rgb(255, 0, 0)) },
                                ByteDecorator::SystemRam(b) => { (b, Color32::from_rgb(0, 255, 0)) },
                                ByteDecorator::AudioRam(b) => { (b, Color32::from_rgb(0, 0, 255)) },
                                ByteDecorator::Vram(b) => { (b, Color32::from_rgb(255, 255, 0)) },
                                ByteDecorator::Framebuffer(b) => { (b, Color32::from_rgb(0, 255, 255)) },
                                ByteDecorator::Aram(b) => { (b, Color32::from_rgb(255, 0, 255)) },
                                ByteDecorator::Unreadable(b) => { (b, Color32::from_rgb(128, 128, 128)) },
                            };
                            let t = RichText::new(format!("{:02X}", byte)).color(color);

                            ui.label(t);
                        });
                    }
                });
            }
        );
    }
}
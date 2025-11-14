use log::{debug, info, warn};
use crate::gametank_bus::{CpuBus};

#[derive(Debug)]
pub struct Blitter {
    // start_time: Instant,

    src_y: u8,
    dst_y: u8,
    height: u8,
    flip_y: bool,

    src_x: u8,
    dst_x: u8,
    width: u8,
    flip_x: bool,

    offset_x: u8,
    offset_y: u8,

    color_fill: bool,

    color: u8,
    blitting: bool,
    cycles: i32,
    pub irq_trigger: bool,
}

impl Blitter {
    pub fn default() -> Self {
        Self {
            src_y: 0,
            dst_y: 0,
            height: 0,
            flip_y: false,
            src_x: 0,
            dst_x: 0,
            width: 0,
            flip_x: false,
            offset_x: 0,
            offset_y: 0,
            color_fill: false,
            color: 0,
            blitting: false,
            cycles: 0,
            irq_trigger: false,
        }
    }

    pub fn clear_irq_trigger(&mut self) -> bool {
        let result = self.irq_trigger;
        self.irq_trigger = false;
        result
    }

    pub fn cycle(&mut self, bus: &mut CpuBus) {
        // debug!(target: "blitter", "{:?}", self);

        let (bit_start, start_addressed) = bus.blitter.start.read_once();
        if start_addressed {
            self.irq_trigger = false;
        }

        // load y at blitter start
        if !self.blitting && bit_start {
            self.src_y = bus.blitter.gy;
            self.dst_y = bus.blitter.vy;
            self.height = bus.blitter.height & 0b01111111;
            self.flip_y = bus.blitter.height & 0b10000000 != 0;
            self.color = !bus.blitter.color;
            self.color_fill = bus.system_control.dma_flags.dma_colorfill_enable();
            self.blitting = true;
            self.cycles = 0;

            // latch for first line
            self.src_x = bus.blitter.gx;
            self.dst_x = bus.blitter.vx;
            self.width = bus.blitter.width & 0b01111111;
            self.flip_x = bus.blitter.width & 0b10000000 != 0;


            debug!(target: "blitter", "starting blit from ({}, {}):({}, {}) page {} at ({}, {}); color mode {}, gcarry {}",
                bus.blitter.gx, bus.blitter.gy,
                bus.blitter.width, bus.blitter.height,
                bus.system_control.banking_register.vram_page(),
                bus.blitter.vx, bus.blitter.vy,
                bus.system_control.dma_flags.dma_colorfill_enable(),
                bus.system_control.dma_flags.dma_gcarry(),
            );
        }

        if !self.blitting {
            return
        }

        // don't update params during a blit line
        if self.offset_x == 0 {
            self.src_y = bus.blitter.gy;
            self.dst_y = bus.blitter.vy;
            self.height = bus.blitter.height & 0b01111111;
            self.flip_y = bus.blitter.height & 0b10000000 != 0;
        }

        if self.offset_x >= self.width {
            self.offset_x = 0;
            self.offset_y += 1;
        }

        if self.offset_y >= self.height {
            self.offset_y = 0;

            self.blitting = false;
            // bus.blitter.start = 0;
            debug!("blit complete, copied {} pixels", self.cycles);
            if bus.system_control.dma_flags.dma_irq() {
                self.irq_trigger = true;
            }
            return
        }


        self.cycles += 1;

        // if blitter is disabled, counters continue but no write occurs
        if !bus.system_control.dma_flags.dma_enable() {
            debug!(target: "blitter", "blit cycle skipped; dma access disabled. dma flags: {:08b}", bus.system_control.dma_flags.0);
            self.offset_x += 1;
            return
        }

        // get the next color to write
        let color = if self.color_fill {
            self.color
        } else {
            // select the page, this makes sense
            let vram_page = bus.system_control.banking_register.vram_page() as usize;

            // ok, src_x and src_y, that makes sense
            let mut src_x_mod = self.src_x;
            let mut src_y_mod = self.src_y;

            let mut blit_src_x;
            let mut blit_src_y;

            if self.flip_x {
                src_x_mod = !src_x_mod;
                blit_src_x = src_x_mod.wrapping_sub(self.offset_x) as usize;
            } else {
                blit_src_x = (src_x_mod.wrapping_add(self.offset_x)) as usize;
            }

            if self.flip_y {
                src_y_mod = !src_y_mod;
                blit_src_y = (src_y_mod.wrapping_sub(self.offset_y)) as usize;
            } else {
                blit_src_y = (src_y_mod.wrapping_add(self.offset_y)) as usize;
            }

            // if gcarry is turned off, blits should tile 16x16
            if !bus.system_control.dma_flags.dma_gcarry() {
                blit_src_x = (src_x_mod.wrapping_add(self.offset_x % 16)) as usize;
                blit_src_y = (src_y_mod.wrapping_add(self.offset_y % 16)) as usize;
            }

            let mut quad = 0;
            if blit_src_x >= 128 {
                quad += 128*128 - 128;
            }
            if blit_src_y >= 128 {
                quad += 128*128;
            }

            bus.vram_banks[vram_page][blit_src_x + blit_src_y*128 + quad]
        };

        let out_x = self.dst_x.wrapping_add(self.offset_x) as usize;
        let out_y = self.dst_y.wrapping_add(self.offset_y) as usize;
        let out_fb = if bus.system_control.get_framebuffer_out() == 1 {
            0
        } else {
            1
        };

        if out_x >= 128 || out_y >= 128 {
            self.offset_x = self.offset_x.wrapping_add(1);
            return
        }

        // write to active framebuffer, if not transparent
        if bus.system_control.dma_flags.dma_opaque() || color != 0 {
            bus.framebuffers[out_fb].borrow_mut()[out_x + out_y*128] = color;
        }

        // increment x offset
        self.offset_x = self.offset_x.wrapping_add(1);
    }
}

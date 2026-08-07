use log::warn;


#[derive(Debug, Copy, Clone)]
pub struct BlitStart {
    pub write: u8,
    pub addressed: bool,
}

impl BlitStart {
    #[inline(always)]
    pub fn read_once(&mut self) -> (bool, bool) {
        let (data, addressed)  = (self.write & 1 == 1, self.addressed);
        self.write = 0;
        self.addressed = false;
        (data, addressed)
    }
}

#[derive(Debug)]
pub struct BlitterRegisters {
    pub vx: u8,
    pub vy: u8,
    pub gx: u8,
    pub gy: u8,
    pub width: u8,
    pub height: u8,
    pub start: BlitStart,
    pub color: u8,
}

impl BlitterRegisters {
    #[inline(always)]
    pub fn vram_quadrant(&self) -> usize {
        let mut quadrant = 0;

        if self.gx >= 128 {
            quadrant += 1
        }

        if self.gy >= 128 {
            quadrant += 2
        }

        quadrant
    }

    #[inline(always)]
    pub fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            0x4006 => { self.start.addressed = true; }
            _ => {} //warn!("Attempted to read from unreadable memory at: ${:02X}", address)
        }

        0
    }

    #[inline(always)]
    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x4000 => { self.vx = data }
            0x4001 => { self.vy = data }
            0x4002 => { self.gx = data }
            0x4003 => { self.gy = data }
            0x4004 => { self.width = data }
            0x4005 => { self.height = data }
            0x4006 => { self.start.write = data; self.start.addressed = true; }
            0x4007 => { self.color = data }
            _ => {}
        }
    }
}
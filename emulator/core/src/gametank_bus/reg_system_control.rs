use log::{debug, warn};
use crate::inputs::GamePad;
use crate::gametank_bus::reg_etc::{BankingRegister, BlitterFlags, GraphicsMemoryMap};

pub const VIA_IORB: usize    = 0x0;
pub const VIA_IORA: usize    = 0x1;
pub const VIA_DDRB: usize   = 0x2;
pub const VIA_DDRA: usize   = 0x3;
pub const VIA_T1CL: usize   = 0x4;
pub const VIA_T1CH: usize   = 0x5;
pub const VIA_T1LL: usize   = 0x6;
pub const VIA_T1LH: usize   = 0x7;
pub const VIA_T2CL: usize   = 0x8;
pub const VIA_T2CH: usize   = 0x9;
pub const VIA_SR: usize     = 0xA;
pub const VIA_ACR: usize    = 0xB;
pub const VIA_PCR: usize    = 0xC;
pub const VIA_IFR: usize    = 0xD;
pub const VIA_IER: usize    = 0xE;
pub const VIA_ORA_NH: usize = 0xF;

pub const VIA_SPI_BIT_CLK : u8 = 0b00000001;
pub const VIA_SPI_BIT_MOSI: u8 = 0b00000010;
pub const VIA_SPI_BIT_CS  : u8 = 0b00000100;
pub const VIA_SPI_BIT_MISO: u8 = 0b10000000;

#[derive(Debug)]
pub struct SystemControl {
    pub reset_acp: u8,
    pub nmi_acp: u8,

    // has effects on the rest of the system
    pub banking_register: BankingRegister,

    pub via_regs: [u8; 16],

    pub audio_enable_sample_rate: u8,
    pub dma_flags: BlitterFlags,

    pub gamepads: [GamePad; 2],

    pub port1_paddle_val: u8
}

impl SystemControl {
    #[inline(always)]
    pub fn get_ram_bank(&self) -> usize {
        self.banking_register.ram_bank() as usize
    }

    #[inline(always)]
    pub fn get_graphics_memory_map(&self) -> GraphicsMemoryMap {
        if self.dma_flags.dma_enable() { // 1 is blitter enabled
            return GraphicsMemoryMap::BlitterRegisters
        }

        if self.dma_flags.dma_cpu_to_vram() {
            return GraphicsMemoryMap::FrameBuffer
        }

        return GraphicsMemoryMap::VRAM
    }

    #[inline(always)]
    pub fn acp_enabled(&self) -> bool {
        (self.audio_enable_sample_rate & 0b1000_0000) != 0
    }
    #[inline(always)]
    pub fn clear_acp_reset(&mut self) -> bool {
        let reset = self.reset_acp & 0b0000_0001;
        self.reset_acp = 0;
        reset == 1
    }

    #[inline(always)]
    pub fn clear_acp_nmi(&mut self) -> bool {
        let nmi = self.nmi_acp & 0b0000_0001;
        self.nmi_acp = 0;
        nmi == 1
    }

    /// Timer divisor derived from $2006. Bit 7 is the ACP enable, not part of the rate.
    #[inline(always)]
    pub fn sample_divisor(&self) -> u32 {
        let low7 = (self.audio_enable_sample_rate & 0x7F) as u32;
        2 * low7 + 1 + (low7 & 1)
    }

    #[inline(always)]
    pub fn sample_rate(&self) -> u8 {
        self.audio_enable_sample_rate
    }

    #[inline(always)]
    pub fn get_framebuffer_out(&self) -> usize {
        self.dma_flags.dma_page_out() as usize
    }

    #[inline(always)]
    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x2000 => { self.reset_acp = data }
            0x2001 => { self.nmi_acp = data }
            0x2005 => {
                debug!("setting banking register to {:08b}", data);
                self.banking_register.0 = data
            }
            0x2006 => { self.audio_enable_sample_rate = data }
            0x2007 => { self.dma_flags.0 = data }
            _ => {
                warn!("Attempted to write read-only memory at: ${:02X}", address);
            }
        }
    }

    #[inline(always)]
    pub fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            0x2008 => {
                self.read_gamepad_byte(true)
            }
            0x2009 => {
                self.read_gamepad_byte(false)
            }
            _ => {
                // warn!("Attempted to read from unreadable memory at: ${:02X}", address);
                0
            }
        }
    }

    #[inline(always)]
    pub fn peek_byte(&self, address: u16) -> u8 {
        match address {
            0x2008 => {
                self.peek_gamepad_byte(true)
            }
            0x2009 => {
                self.peek_gamepad_byte(false)
            }
            _ => {
                0
            }
        }
    }

    #[inline(always)]
    pub fn read_gamepad_byte(&mut self, port_1: bool) -> u8 {
        let byte = self.peek_gamepad_byte(port_1);

        self.gamepads[port_1 as usize].port_select = false;
        self.gamepads[(!port_1) as usize].port_select = !self.gamepads[(!port_1) as usize].port_select;

        byte
    }


    #[inline(always)]
    pub fn peek_gamepad_byte(&self, port_1: bool) -> u8 {
        let gamepad = &self.gamepads[(!port_1) as usize];
        let paddle_val = self.port1_paddle_val;
        let mut byte = 255;
        if !gamepad.port_select {
            byte &= !((gamepad.start as u8) << 5);
            byte &= !((gamepad.a as u8) << 4);

            byte &= !((gamepad.x as u8) << 1); //paddle bit
            byte &= !((gamepad.y as u8) << 2); //paddle bit
            byte &= !((gamepad.z as u8) << 3); //paddle bit
            byte &= !((gamepad.mode as u8) << 0); //paddle bit

        } else {
            byte &= !((gamepad.c as u8) << 5);
            byte &= !((gamepad.b as u8) << 4);
            byte &= !((gamepad.up as u8) << 3);
            byte &= !((gamepad.down as u8) << 2);
            byte &= !((gamepad.left as u8) << 1);
            byte &= !((gamepad.right as u8) << 0);
        }
        byte
    }

    #[inline(always)]
    pub fn apply_paddle_delta(&mut self, delta: i8) {
        self.port1_paddle_val = self.port1_paddle_val.saturating_add_signed(delta);
        let bits = !self.port1_paddle_val;

        let gamepad = &mut self.gamepads[0];

        // Pins evaluated during Read 1 (port_select = true):
        gamepad.up    = (bits & (1 << 0)) != 0; // Bit 0 -> UP pin
        gamepad.down  = (bits & (1 << 1)) != 0; // Bit 1 -> DOWN pin
        gamepad.left  = (bits & (1 << 2)) != 0; // Bit 2 -> Left pin
        gamepad.right = (bits & (1 << 3)) != 0; // Bit 3 -> Right pin

        // Pins evaluated during Read 0 (port_select = false):
        gamepad.x     = (bits & (1 << 4)) != 0; // Bit 4 -> X pin
        gamepad.y     = (bits & (1 << 5)) != 0; // Bit 5 -> Y pin
        gamepad.z     = (bits & (1 << 6)) != 0; // Bit 6 -> Z pin
        gamepad.mode  = (bits & (1 << 7)) != 0; // Bit 7 -> Mode pin

    }
}
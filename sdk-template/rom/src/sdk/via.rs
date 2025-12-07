//! # VIA - ROM Banking
//!
//! The VIA handles ROM bank switching for accessing the 2MB cartridge ROM.
//!
//! ## ROM Banking
//!
//! The GameTank has 2MB of ROM divided into 128 banks (0-127), each 16KB.
//! Bank 127 is always visible at `$C000-$FFFF` (where your main code lives).
//! Use the VIA to switch which bank appears at `$8000-$BFFF`:
//!
//! ```ignore
//! let via = unsafe { Via::new() };
//!
//! // Switch to bank 10 to access data there
//! via.change_rom_bank(10);
//!
//! // Now data in bank 10 is accessible
//! ```
//!
//! ## Placing Data in Banks
//!
//! Use link sections to put data in specific banks:
//!
//! ```ignore
//! // This array will be in ROM bank 10
//! #[unsafe(link_section = ".rodata.bank10")]
//! static LEVEL_DATA: [u8; 8192] = include_bytes!("level1.bin");
//!
//! // Switch to bank 10 before accessing
//! via.change_rom_bank(10);
//! let first_byte = LEVEL_DATA[0];  // Now accessible!
//! ```
//!
//! **Tip for future carts:** Use banks 128-255 instead of 0-127 for compatibility
//! with battery-backed RAM cartridges (they use bit 7 to select RAM vs ROM).

use bit_field::BitField;
use volatile_register::{RW, WO};

#[repr(C, packed)]
pub struct Via {
    pub iorb: RW<u8>, // input/output register b
    pub iora: RW<u8>, // input/output register a
    pub ddrb: WO<u8>, //
    pub ddra: WO<u8>,
    pub t1cl: WO<u8>,
    pub t1ch: WO<u8>,
    pub t2cl: WO<u8>,
    pub t2ch: WO<u8>,
    pub sr: WO<u8>,
    pub acr: WO<u8>,
    pub pcr: WO<u8>,
    pub ifr: WO<u8>,
    pub era: WO<u8>,
    pub iora_nh: WO<u8>,
}

impl Via {
    pub unsafe fn new() -> &'static mut Via {
        unsafe { &mut *(0x2800 as *mut Via) }
    }

    #[inline(always)]
    pub fn change_rom_bank(&mut self, banknum: u8) {
        unsafe {
            self.ddra.write(0b00000111); // I have no idea what this does
            self.iora.write(0);
            self.iora.write((banknum.get_bit(7) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(6) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(5) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(4) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(3) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(2) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(1) as u8) << 1); // this line could be simplified to a mask, but we don't for consistency
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write((banknum.get_bit(0) as u8) << 1);
            self.iora.write(*self.iora.read().set_bit(0, true));
            self.iora.write(*self.iora.read().set_bit(2, true));
            self.iora.write(0);
        }
    }

    pub fn profiler_start(&mut self, id: u8) {
        unsafe { self.iorb.write(0x80) };
        unsafe { self.iorb.write(id) };
    }

    pub fn profiler_end(&mut self, id: u8) {
        unsafe { self.iorb.write(0x80) };
        unsafe { self.iorb.write(id | 0x40) };
    }
}

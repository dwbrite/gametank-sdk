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

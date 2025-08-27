const fn make_volume_table(num: i16, den: i16) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0;

    while i < 256 {    
        let v = ((((i as i16 - 128) * num) / den) + 128) as u8;
        table[i] = v as u8;
        i += 1;
    }
    table
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".const.vol")]
pub static VT: [[u8; 256];4] = [
    make_volume_table(1, 1),
    make_volume_table(7, 8),
    make_volume_table(3, 4),
    make_volume_table(5, 8),
];


pub const VOLUME: [Volume; 17] = [
    Volume { volume_ptr: &VT[3], vol_shift: 4 },

    Volume { volume_ptr: &VT[0], vol_shift: 3 }, Volume { volume_ptr: &VT[1], vol_shift: 3 },
    Volume { volume_ptr: &VT[2], vol_shift: 3 }, Volume { volume_ptr: &VT[3], vol_shift: 3 },

    Volume { volume_ptr: &VT[0], vol_shift: 2 }, Volume { volume_ptr: &VT[1], vol_shift: 2 },
    Volume { volume_ptr: &VT[2], vol_shift: 2 }, Volume { volume_ptr: &VT[3], vol_shift: 2 },
        
    Volume { volume_ptr: &VT[0], vol_shift: 1 }, Volume { volume_ptr: &VT[1], vol_shift: 1 },
    Volume { volume_ptr: &VT[2], vol_shift: 1 }, Volume { volume_ptr: &VT[3], vol_shift: 1 },

    Volume { volume_ptr: &VT[0], vol_shift: 0 }, Volume { volume_ptr: &VT[1], vol_shift: 0 },
    Volume { volume_ptr: &VT[2], vol_shift: 0 }, Volume { volume_ptr: &VT[3], vol_shift: 0 },
];


#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Volume {
    volume_ptr: *const [u8; 256],
    vol_shift: u8,
}

#[inline(always)]
#[unsafe(no_mangle)]
fn vol_shift0(s: u8) -> u8 { s }
#[inline(always)]
#[unsafe(no_mangle)]
fn vol_shift1(s: u8) -> u8 {
    // if s < 128 { 128 - ((128 - s) >> 1) } else { 128 + ((s - 128) >> 1) }
    128u8.wrapping_add(((s.wrapping_sub(128)) as i8 >> 1) as u8)
}
#[inline(always)]
#[unsafe(no_mangle)]
fn vol_shift2(s: u8) -> u8 {
    // if s < 128 { 128 - ((128 - s) >> 2) } else { 128 + ((s - 128) >> 2) }
    128u8.wrapping_add(((s.wrapping_sub(128)) as i8 >> 2) as u8)
}
#[inline(always)]
#[unsafe(no_mangle)]
fn vol_shift3(s: u8) -> u8 {
    // if s < 128 { 128 - ((128 - s) >> 3) } else { 128 + ((s - 128) >> 3) }
    128u8.wrapping_add(((s.wrapping_sub(128)) as i8 >> 3) as u8)
}

impl Volume {
    #[inline(always)]
    #[unsafe(no_mangle)]
    pub fn volume(&self, sample: u8) -> u8 {
        let sample = unsafe { *(*self.volume_ptr).get_unchecked(sample as usize) };
        let s = sample;
        
        let mut d= (s.wrapping_sub(128)) as i8;

        for _ in 0..self.vol_shift {
            d >>= 1; // arithmetic right shift by 1 each time
        }

        if self.vol_shift >= 4 {
            return 128
        }

        128u8.wrapping_add(d as u8)

        // match self.vol_shift {
        //     0 => vol_shift0(s),
        //     1 => vol_shift1(s),
        //     2 => vol_shift2(s),
        //     3 => vol_shift3(s),
        //     _ => 128,
        // }
    }
}

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

impl Volume {
    #[inline(always)]
    #[unsafe(no_mangle)]
    pub fn volume(&self, sample: u8) -> u8 {
        // sample Mult LUT 
        let sample = unsafe { *(*self.volume_ptr).get_unchecked(sample as usize) };
        let s = sample;
        
        // re-bias towards 0 (wrapping sub maintains 2s comp for i8 conversion)
        let mut d= (s.wrapping_sub(128)) as i8;

        for _ in 0..self.vol_shift {
            d >>= 1; // divide by 2 for each vol_shift
        }

        // if >= 4 shifts, you want silence
        if self.vol_shift >= 4 {
            return 128
        }

        // re-bias towards 128 center - wrapping add maintains 2s comp for u8 conversion ;)
        128u8.wrapping_add(d as u8)
    }
}

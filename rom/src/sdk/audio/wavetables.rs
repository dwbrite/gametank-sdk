use crate::sdk::audio::pitch_table::{self, midi_inc, MidiNote};


const WAVETABLES: [usize; 8] = [
    0x0600,
    0x0700,
    0x0800,
    0x0900,
    0x0a00,
    0x0b00,
    0x0c00,
    0x0d00,
];

const VOL: [usize; 4] = [
    0x0500,
    0x0400,
    0x0300,
    0x0200,
];

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Volume {
    volume_ptr: usize,
    vol_shift: u8,
}

pub const VOLUME: [Volume; 17] = [
    Volume { volume_ptr: VOL[3], vol_shift: 4 },

    Volume { volume_ptr: VOL[0], vol_shift: 3 }, Volume { volume_ptr: VOL[1], vol_shift: 3 },
    Volume { volume_ptr: VOL[2], vol_shift: 3 }, Volume { volume_ptr: VOL[3], vol_shift: 3 },

    Volume { volume_ptr: VOL[0], vol_shift: 2 }, Volume { volume_ptr: VOL[1], vol_shift: 2 },
    Volume { volume_ptr: VOL[2], vol_shift: 2 }, Volume { volume_ptr: VOL[3], vol_shift: 2 },
        
    Volume { volume_ptr: VOL[0], vol_shift: 1 }, Volume { volume_ptr: VOL[1], vol_shift: 1 },
    Volume { volume_ptr: VOL[2], vol_shift: 1 }, Volume { volume_ptr: VOL[3], vol_shift: 1 },

    Volume { volume_ptr: VOL[0], vol_shift: 0 }, Volume { volume_ptr: VOL[1], vol_shift: 0 },
    Volume { volume_ptr: VOL[2], vol_shift: 0 }, Volume { volume_ptr: VOL[3], vol_shift: 0 },
];

#[repr(C, packed)]
pub struct Voice {
    phase: u16,
    frequency: u16,
    wavetable: usize,
    volume: Volume,
}


// TODO: NOTE type which maps to u16 at compile time
impl Voice {
    pub fn set_tone(&mut self, note: MidiNote) {
        self.frequency = midi_inc(note);
    }

    pub fn set_volume(&mut self, volume: Volume) {
        self.volume = volume;
    }
}


// who cares about aliasing
pub fn voices() -> &'static mut [Voice; 7] {
    unsafe {
        &mut *(0x3041 as *mut [Voice; 7])
    }
}

// [
    //0     37,     37,     42,     42,     46,     46,      51,      56,     56,     60,     65,     70,     74,     79, 
    //1     84,     89,     93,     98,    107,    112,     117,     126,    135,    140,    149,    159,    168,    178, 
    //2    192,    201,    215,    224,    239,    253,     271,     285,    304,    323,    342,    360,    384,    407, 
    //3    431,    454,    482,    510,    543,    576,     609,     646,    684,    726,    768,    815,    862,    913, 
    //4    970,   1026,   1087,   1152,   1223,   1293,    1373,    1452,   1541,   1631,   1729,   1832,   1940,   2057, 
    // 2179, 2310, 2446, 2591, 2746, 2910, 3083, 
    // 3266, 3463, 3669, 3885, 4119, 4363, 4625, 
    // 4897, 5188, 5497, 5825, 6172, 6538, 6927, 
    // 7339, 7775, 8239, 8731, 9251, 9800, 10381, 
    // 10999, 11656, 12349, 13080, 13858, 14683, 
    // 15555, 16483, 17463, 18503, 19600, 20767, 
    // 22004, 23312, 24699, 26166, 27722, 29372, 
    // 31115, 32967, 34930, 37007, 39205, 41539, 
    // 44009, 46624, 49399, 52337, 55449, 58744
    // ]
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
pub fn voices() -> &'static mut [Voice; 8] {
    unsafe {
        &mut *(0x3041 as *mut [Voice; 8])
    }
}

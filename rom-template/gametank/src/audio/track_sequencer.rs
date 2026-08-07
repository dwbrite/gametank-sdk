use super::wavetable_8ch::{VOICE_COUNT, WAVETABLE, voices};

const BEATS: usize = 64;
const CHANNEL_STRIDE: usize = BEATS * 3; // freq_lo[64] + freq_hi[64] + vol[64]

/// Drives the 8-channel wavetable synth from a gt-tracker export.
/// Create one sequencer per track, point it at the `<name>_track` descriptor, then call [`init_voices`] once after loading
/// the firmware and [`tick`] once per frame.
///
/// ```rust,ignore
/// unsafe extern "C" { static mysong_track: u8; }
///
/// let mut sequencer = TrackSequencer::new(unsafe { &mysong_track as *const u8 });
/// sequencer.init_voices();
///
/// loop {
///     unsafe { wait(); }
///     sequencer.tick();
/// }
/// ```
pub struct TrackSequencer {
    track: *const u8,
    beat: u8,
    frame_acc: u16,
    bpm: u16,
    pattern_idx: u8,
    sequence_len: u8,
}

impl TrackSequencer {
    /// Create a sequencer for the given track descriptor.
    pub fn new(track: *const u8) -> Self {
        let bpm = unsafe { (track as *const u16).read_unaligned() };
        let sequence_len = unsafe { *track.add(3) };
        Self {
            track,
            beat: 0,
            frame_acc: 0,
            bpm,
            pattern_idx: 0,
            sequence_len,
        }
    }

    /// Point each voice at its corresponding instrument wavetable and mute all voices.
    pub fn init_voices(&self) {
        let v = voices();
        for i in 0..VOICE_COUNT {
            v[i].set_wavetable(WAVETABLE[i]);
            v[i].set_volume(0);
        }
    }

    /// Advance the sequencer by one frame. Call at 60 fps.
    pub fn tick(&mut self) {
        self.frame_acc += self.bpm;
        if self.frame_acc >= 3600 {
            self.frame_acc -= 3600;
            self.advance_beat();
        }
    }

    fn advance_beat(&mut self) {
        let t = self.track;
        let seq = unsafe { read_u16(t, 4) } as *const u8;
        let pat_table = unsafe { read_u16(t, 6) } as *const u16;
        let evt_table = unsafe { read_u16(t, 8) } as *const u16;

        let seq_idx = self.pattern_idx as usize;
        let pat_idx = unsafe { *seq.add(seq_idx) } as usize;
        let pat = unsafe { read_ptr(pat_table, pat_idx) } as *const u8;

        let beat = self.beat as usize;
        let v = voices();
        for ch in 0..VOICE_COUNT {
            let base = ch * CHANNEL_STRIDE;
            let lo = unsafe { *pat.add(base + beat) } as u16;
            let hi = unsafe { *pat.add(base + BEATS + beat) } as u16;
            let vol = unsafe { *pat.add(base + BEATS * 2 + beat) };
            if lo | hi != 0 {
                v[ch].set_frequency(lo | (hi << 8));
            }
            if vol != 0xFF {
                v[ch].set_volume(vol);
            }
        }

        let evt_list = unsafe { read_ptr(evt_table, pat_idx) } as *const u8;
        let evt_count = unsafe { *evt_list } as usize;
        for e in 0..evt_count {
            let b = 1 + e * 3;
            if unsafe { *evt_list.add(b) } as usize != beat {
                continue;
            }
            match unsafe { *evt_list.add(b + 1) } {
                0x00 => {
                    self.next_pattern();
                    return;
                }
                0x01 => {
                    self.bpm = unsafe { *evt_list.add(b + 2) } as u16;
                }
                _ => {}
            }
        }

        self.beat += 1;
        if (self.beat as usize) >= BEATS {
            self.next_pattern();
        }
    }

    fn next_pattern(&mut self) {
        self.beat = 0;
        self.pattern_idx += 1;
        if self.pattern_idx >= self.sequence_len {
            self.pattern_idx = 0;
        }
        self.bpm = unsafe { read_u16(self.track, 0) };
    }
}

unsafe fn read_u16(base: *const u8, offset: usize) -> u16 {
    (base.add(offset) as *const u16).read_unaligned()
}

unsafe fn read_ptr(table: *const u16, index: usize) -> u16 {
    table.add(index).read_unaligned()
}

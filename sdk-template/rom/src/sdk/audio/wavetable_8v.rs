//! # 8-Voice Wavetable Synthesizer
//!
//! This firmware provides 8 independent voices, each with:
//! - **Note/Frequency** - MIDI notes or raw frequency values
//! - **Volume** - 0 (silent) to 63 (max)
//! - **Wavetable** - One of 8 waveform slots
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rom::sdk::audio::{voices, MidiNote, WAVETABLE};
//!
//! let v = voices();
//!
//! // Play middle C at full volume with the first wavetable
//! v[0].set_note(MidiNote::C4);
//! v[0].set_volume(63);
//! v[0].set_wavetable(WAVETABLE[0]);
//!
//! // Play a chord
//! v[1].set_note(MidiNote::E4);  v[1].set_volume(50);
//! v[2].set_note(MidiNote::G4);  v[2].set_volume(50);
//!
//! // Stop a voice
//! v[0].mute();
//! ```
//!
//! ## Wavetables
//!
//! The firmware has 8 wavetable slots. Use [`WAVETABLE`] to get slot addresses:
//!
//! ```rust,ignore
//! v[0].set_wavetable(WAVETABLE[0]);  // First waveform
//! v[1].set_wavetable(WAVETABLE[1]);  // Second waveform
//! ```
//!
//! You can load custom waveforms (256 bytes each) into audio RAM:
//!
//! ```rust,ignore
//! // Wavetable 0 is at $3400, wavetable 1 at $3500, etc.
//! let my_wave: [u8; 256] = generate_sine();
//! console.audio[0x400..0x500].copy_from_slice(&my_wave);
//! ```

use crate::sdk::audio::pitch_table::{midi_inc, MidiNote};

/// Base address for voice registers (CPU-side address, ACP RAM at 0x3000)
pub const VOICE_BASE: usize = 0x3041;
/// Number of bytes per voice
pub const VOICE_SIZE: usize = 7;
/// Number of voices
pub const VOICE_COUNT: usize = 8;

/// Base address for wavetables in ACP RAM (CPU-side)
pub const WAVETABLE_BASE: usize = 0x3400;
/// Size of each wavetable in bytes
pub const WAVETABLE_SIZE: usize = 256;
/// Number of wavetables available
pub const WAVETABLE_COUNT: usize = 8;

/// Wavetable slot addresses (CPU-side)
pub const WAVETABLE: [u16; WAVETABLE_COUNT] = [
    0x0400, 0x0500, 0x0600, 0x0700,
    0x0800, 0x0900, 0x0A00, 0x0B00,
];

/// A single synthesizer voice.
///
/// This struct is laid out to match the ACP firmware's memory layout exactly.
/// All fields are little-endian as expected by the 6502.
#[repr(C, packed)]
pub struct Voice {
    /// Phase accumulator (16.8 fixed point, high byte indexes wavetable)
    phase: u16,
    /// Frequency increment added to phase each sample
    frequency: u16,
    /// Pointer to 256-byte wavetable in ACP RAM
    wavetable: u16,
    /// Volume level (0 = silence, 63 = max)
    volume: u8,
}

impl Voice {
    /// Set the voice frequency from a MIDI note number.
    #[inline]
    pub fn set_note(&mut self, note: MidiNote) {
        self.frequency = midi_inc(note);
    }

    /// Set the voice frequency directly as a 16-bit increment value.
    /// 
    /// Use `pitch_table::midi_inc()` to convert from MIDI notes,
    /// or calculate directly: `inc = (freq_hz * 65536) / SAMPLE_RATE`
    #[inline]
    pub fn set_frequency(&mut self, freq_inc: u16) {
        self.frequency = freq_inc;
    }

    /// Set the volume level (0 = silence, 63 = maximum).
    /// 
    /// Values above 63 may cause clipping/distortion.
    #[inline]
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    /// Set which wavetable this voice uses.
    /// 
    /// Pass the ACP-side address (e.g., `WAVETABLE[0]` = 0x0400).
    #[inline]
    pub fn set_wavetable(&mut self, wavetable_addr: u16) {
        self.wavetable = wavetable_addr;
    }

    /// Silence this voice immediately.
    #[inline]
    pub fn mute(&mut self) {
        self.volume = 0;
    }

    /// Reset the phase accumulator to zero (useful for hard sync effects).
    #[inline]
    pub fn reset_phase(&mut self) {
        self.phase = 0;
    }

    /// Get the current volume level.
    #[inline]
    pub fn get_volume(&self) -> u8 {
        self.volume
    }
}

/// Get a mutable reference to all 8 voices.
///
/// # Safety
/// This function creates a mutable reference to memory-mapped hardware.
/// The caller must ensure exclusive access to the voice registers.
#[inline]
pub fn voices() -> &'static mut [Voice; VOICE_COUNT] {
    unsafe { &mut *(VOICE_BASE as *mut [Voice; VOICE_COUNT]) }
}

/// Get a mutable reference to a single voice by index (0-7).
///
/// # Panics
/// Panics if `index >= 8`.
#[inline]
pub fn voice(index: usize) -> &'static mut Voice {
    assert!(index < VOICE_COUNT, "voice index out of range");
    unsafe { &mut *((VOICE_BASE + index * VOICE_SIZE) as *mut Voice) }
}

/// Silence all voices.
#[inline]
pub fn mute_all() {
    let v = voices();
    for voice in v.iter_mut() {
        voice.mute();
    }
}

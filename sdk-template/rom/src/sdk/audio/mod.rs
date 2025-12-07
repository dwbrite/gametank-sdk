//! # Audio
//!
//! The GameTank uses a dedicated 6502 coprocessor for audio synthesis.
//! This module provides the audio firmware and a high-level interface.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rom::sdk::audio::{FIRMWARE, voices, MidiNote, WAVETABLE};
//!
//! // Initialize audio (do once at startup)
//! console.sc.set_audio(0);                    // Disable while loading
//! console.audio.copy_from_slice(FIRMWARE);    // Load firmware
//! console.sc.set_audio(0xFF);                 // Enable at ~14kHz
//!
//! // Play a note
//! let v = voices();
//! v[0].set_note(MidiNote::C4);
//! v[0].set_volume(63);
//! v[0].set_wavetable(WAVETABLE[0]);
//! ```
//!
//! ## Playing Music
//!
//! The wavetable synth gives you 8 voices. Each voice has:
//! - **Note/Frequency** - Set with [`Voice::set_note`](wavetable_8v::Voice::set_note) or raw frequency
//! - **Volume** - 0 (silent) to 63 (max)
//! - **Wavetable** - Which of 8 waveforms to use
//!
//! ```rust,ignore
//! let v = voices();
//!
//! // Play a C major chord
//! v[0].set_note(MidiNote::C4);  v[0].set_volume(50);
//! v[1].set_note(MidiNote::E4);  v[1].set_volume(50);
//! v[2].set_note(MidiNote::G4);  v[2].set_volume(50);
//!
//! // Stop a voice
//! v[0].mute();
//! ```
//!
//! ## Custom Wavetables
//!
//! You can load custom 256-byte waveforms into the wavetable slots:
//!
//! ```rust,ignore
//! // Wavetables live at $3400-$3BFF in audio RAM (8 × 256 bytes)
//! let waveform: [u8; 256] = make_sine_wave();
//! console.audio[0x400..0x500].copy_from_slice(&waveform);
//! ```
//!
//! ## Audio Firmware
//!
//! Enable a firmware via Cargo features:
//! - `audio-wavetable-8v` - 8-voice wavetable synth (default, recommended)
//!
//! The firmware runs on the Audio Coprocessor at ~14kHz sample rate,
//! with about 660 CPU cycles available per sample for synthesis.

// Audio firmware binary - selected via Cargo.toml features
#[cfg(feature = "audio-wavetable-8v")]
pub static FIRMWARE: &[u8; 4096] = include_bytes!("../../../../audiofw/wavetable-8v.bin");

#[cfg(feature = "audio-fm-4op")]
pub static FIRMWARE: &[u8; 4096] = include_bytes!("../../../../audiofw/fm-4op.bin");

// Audio interface modules - selected via Cargo.toml features
#[cfg(feature = "audio-wavetable-8v")]
pub mod wavetable_8v;
#[cfg(feature = "audio-wavetable-8v")]
pub use wavetable_8v::*;

// Shared
pub mod pitch_table;
pub use pitch_table::MidiNote;


//! # Audio
//!
//! The GameTank uses a dedicated 6502 coprocessor for audio synthesis.
//! This module provides the audio firmware and a high-level interface.
//!
//! ## Quick Start (Wavetable)
//!
//! ```rust,ignore
//! use gametank::audio::{FIRMWARE, voices, MidiNote, WAVETABLE};
//!
//! console.sc.set_audio(0);
//! console.audio.copy_from_slice(FIRMWARE);
//! console.sc.set_audio(0xFF);
//!
//! let v = voices();
//! v[0].set_note(MidiNote::C4);
//! v[0].set_volume(63);
//! v[0].set_wavetable(WAVETABLE[0]);
//! ```
//!
//! ## Quick Start (FM)
//!
//! ```rust,ignore
//! use gametank::audio::{FIRMWARE, channels, MidiNote};
//! use gametank::audio::fm_4ch::instruments::PIANO;
//!
//! console.sc.set_audio(0);
//! console.audio.copy_from_slice(FIRMWARE);
//! console.sc.set_audio(0xFF);
//!
//! let ch = channels();
//! ch[0].load_instrument(&PIANO);
//! ch[0].set_note(MidiNote::C4);
//! ch[0].note_on();
//! // call ch[0].tick() once per frame to advance the ADSR envelope
//! ```
//!

// Audio firmware binary - selected via Cargo.toml features
#[cfg(feature = "audio-wavetable-8ch")]
pub static FIRMWARE: &[u8; 4096] = include_bytes!("../../audiofw/wavetable-8ch.bin");

#[cfg(feature = "audio-fm-4ch")]
pub static FIRMWARE: &[u8; 4096] = include_bytes!("../../audiofw/fm-4ch.bin");

// Audio interface modules - selected via Cargo.toml features
#[cfg(feature = "audio-wavetable-8ch")]
pub mod wavetable_8ch;
#[cfg(feature = "audio-wavetable-8ch")]
pub use wavetable_8ch::*;

#[cfg(feature = "audio-wavetable-8ch")]
pub mod track_sequencer;
#[cfg(feature = "audio-wavetable-8ch")]
pub use track_sequencer::TrackSequencer;

#[cfg(feature = "audio-fm-4ch")]
pub mod fm_4ch;
#[cfg(feature = "audio-fm-4ch")]
pub use fm_4ch::*;

// Shared
pub mod pitch_table;
pub use pitch_table::MidiNote;

use crate::sdk::audio::pitch_table::MidiNote;

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

enum AudioMessage {
    NoteOn(MidiNote),
    NoteOff,
    Delay(u8),
}

struct AudioData {
    
}


impl AudioData {
    fn process_audio(ticks: u16) {

    }
}


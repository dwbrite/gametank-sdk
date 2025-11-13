use crate::sdk::audio::pitch_table::MidiNote;

pub mod wavetables;
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


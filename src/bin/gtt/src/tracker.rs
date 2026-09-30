use serde::{Deserialize, Serialize};

pub const PATTERN_TABLE_WIDTH: u16 = 104;
pub const PATTERN_BEATS: usize = 64;

pub type Pattern = [[Beat; PATTERN_BEATS]; 8];

pub fn empty_pattern() -> Pattern {
    std::array::from_fn(|_| std::array::from_fn(|_| Beat::default()))
}

pub const FX_ID_NONE: u8 = 0;
pub const FX_ID_INSTRUMENT: u8 = 1;
pub const FX_ID_ARPEGGIO: u8 = 2;
pub const FX_ID_PITCH_UP: u8 = 3;
pub const FX_ID_PITCH_DOWN: u8 = 4;
pub const FX_ID_FADE_IN: u8 = 5;
pub const FX_ID_FADE_OUT: u8 = 6;
pub const FX_ID_TREMBLE: u8 = 7;
pub const MAX_FX_ID: u8 = FX_ID_TREMBLE;
pub const MAX_INSTRUMENT_INDEX: u8 = 10;
pub const NOISE_INSTRUMENT: u8 = MAX_INSTRUMENT_INDEX;
pub const FX_TICKS_MAX: u8 = 0x3B;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Beat {
    pub cmd_list: Vec<ChannelCmd>,
    pub sqc: Option<SequencerCmd>,
}

impl Beat {
    // Returns the active channel Fx command (if any) as (fx_id, x, y)
    pub fn fx(&self) -> Option<(u8, u8, Option<u8>)> {
        self.cmd_list.iter().find_map(|c| match c {
            ChannelCmd::Instrument(x, y) => Some((FX_ID_INSTRUMENT, *x, *y)),
            ChannelCmd::Arpeggio(x, y) => Some((FX_ID_ARPEGGIO, *x, *y)),
            ChannelCmd::PitchUp(x) => Some((FX_ID_PITCH_UP, *x, None)),
            ChannelCmd::PitchDown(x) => Some((FX_ID_PITCH_DOWN, *x, None)),
            ChannelCmd::FadeIn(x) => Some((FX_ID_FADE_IN, *x, None)),
            ChannelCmd::FadeOut(x) => Some((FX_ID_FADE_OUT, *x, None)),
            ChannelCmd::Tremble(x) => Some((FX_ID_TREMBLE, *x, None)),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequencerCmd {
    // Advance,
    // Beat(u8),
    // Load(u8, u16),
    // Pattern(u8),
    Stop,
    Tempo(u8),
    FxSpeed(u8),
    FlowCount(u8),
    CountJump(u8, u8),
    Jump(u8, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelCmd {
    Note(String),
    NoteOff,
    Phase(u16),
    Instrument(u8, Option<u8>),
    Arpeggio(u8, Option<u8>),
    PitchUp(u8),
    PitchDown(u8),
    FadeIn(u8),
    FadeOut(u8),
    Tremble(u8),
    Volume(u8),
    Wavetable(u16),
}

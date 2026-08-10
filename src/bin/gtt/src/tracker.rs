use serde::{Deserialize, Serialize};

pub const PATTERN_TABLE_WIDTH: u16 = 114;

pub type Pattern = [[Beat; 64]; 9];

pub fn empty_pattern() -> Pattern {
    std::array::from_fn(|_| std::array::from_fn(|_| Beat::default()))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Beat {
    pub cmd_list: Vec<ChannelCmd>,
    pub sqc_list: Vec<SequencerCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequencerCmd {
    Advance,
    Beat(u8),
    Load(u8, u16),
    Pattern(u8),
    Stop,
    Tempo(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelCmd {
    Note(String),
    Phase(u16),
    SlidePitch(u8, i16),
    SlideVol(u8, i16),
    StopPSlide,
    StopVSlide,
    Tremolo(u8, u8),
    Vibrato(u8, u8),
    Volume(u8),
    Wavetable(u16),
}

use std::io;
use std::path::Path;

use crate::file::{NUM_INSTRUMENTS, TrackerFile};
use crate::sample_rate::{SAMPLE_RATE_REG, sample_rate_reg_to_hz};
use crate::tracker::{
    ChannelCmd, FX_ID_ARPEGGIO, FX_ID_FADE_IN, FX_ID_FADE_OUT, FX_ID_INSTRUMENT, FX_ID_NONE,
    FX_ID_PITCH_DOWN, FX_ID_PITCH_UP, FX_ID_TREMBLE, SequencerCmd,
};

const CHANNELS: usize = 7;

const VOL_NO_CHANGE: u8 = 0xFF;

const TRACK_HEADER_SIZE: usize = 5;

const ARP_NO_THIRD_NOTE: u8 = 0xFF;

fn note_to_freq_inc(hz: f64, sample_rate_hz: f64) -> u16 {
    ((hz / sample_rate_hz) * 65536.0).round().min(65535.0) as u16
}

// Resolve the phase increment of the note u8 `steps` above `note_name`
// Returns `None` if note_name isn't found in the tuning table
fn arp_offset_freq_inc(
    file: &TrackerFile,
    note_name: &str,
    steps: u8,
    sample_rate_hz: f64,
) -> Option<u16> {
    let idx = file.tuning.notes.get_index_of(note_name)?;
    let target_idx = (idx + steps as usize).min(file.tuning.notes.len().saturating_sub(1));
    let (_, &hz) = file.tuning.notes.get_index(target_idx)?;
    Some(note_to_freq_inc(hz, sample_rate_hz))
}

// Resolve the phase increment of the note u8 `steps` below `note_name`
// Returns `None` if note_name isn't found in the tuning table
fn pitch_down_offset_freq_inc(
    file: &TrackerFile,
    note_name: &str,
    steps: u8,
    sample_rate_hz: f64,
) -> Option<u16> {
    let idx = file.tuning.notes.get_index_of(note_name)?;
    let target_idx = idx.saturating_sub(steps as usize);
    let (_, &hz) = file.tuning.notes.get_index(target_idx)?;
    Some(note_to_freq_inc(hz, sample_rate_hz))
}

// SequencerCmd encoding
const SEQ_CMD_NONE: u8 = 0;
const SEQ_CMD_STOP: u8 = 1;
const SEQ_CMD_TEMPO: u8 = 2;
const SEQ_CMD_FX_SPEED: u8 = 3;
const SEQ_CMD_FLOW_COUNT: u8 = 4;
const SEQ_CMD_COUNT_JUMP: u8 = 5;
const SEQ_CMD_JUMP: u8 = 6;

fn bake_pattern(
    file: &TrackerFile,
    pattern_idx: usize,
    beats: usize,
    sample_rate_hz: f64,
) -> Vec<u8> {
    let pattern = file.current_pattern(pattern_idx as u8);
    let channel_stride = beats * 10;
    let seq_cmd_base = 1 + CHANNELS * channel_stride;
    let mut out = vec![0u8; seq_cmd_base + beats * 3];
    out[0] = beats as u8;

    for ch in 0..CHANNELS {
        let mut cur_freq: u16 = 0;
        let mut cur_note_name: Option<String> = None;
        let mut remembered_vol: u8 = 0;
        let mut muted = false;

        for beat in 0..beats {
            let row = &pattern[ch + 1][beat];

            let mut explicit_vol: Option<u8> = None;
            let mut has_note = false;
            let mut has_note_off = false;
            let mut fx_id = FX_ID_NONE;
            let mut fx_x = 0u8;
            let mut fx_y = 0u8;

            // Process ChannelCmds
            for cmd in &row.cmd_list {
                match cmd {
                    ChannelCmd::Note(name) => {
                        has_note = true;
                        if let Some(&hz) = file.tuning.notes.get(name.as_str()) {
                            cur_freq = note_to_freq_inc(hz, sample_rate_hz);
                            cur_note_name = Some(name.clone());
                        }
                    }
                    ChannelCmd::Phase(inc) => {
                        cur_freq = *inc;
                        cur_note_name = None;
                    }
                    ChannelCmd::Volume(v) => {
                        explicit_vol = Some((*v).min(63));
                    }
                    ChannelCmd::NoteOff => {
                        has_note_off = true;
                    }
                    ChannelCmd::Instrument(idx, y) => {
                        fx_id = FX_ID_INSTRUMENT;
                        fx_x = *idx;
                        fx_y = y.unwrap_or(0);
                    }
                    ChannelCmd::Arpeggio(x, y) => {
                        fx_id = FX_ID_ARPEGGIO;
                        fx_x = *x;
                        fx_y = y.unwrap_or(ARP_NO_THIRD_NOTE);
                    }
                    ChannelCmd::PitchUp(x) => {
                        fx_id = FX_ID_PITCH_UP;
                        fx_x = *x;
                    }
                    ChannelCmd::PitchDown(x) => {
                        fx_id = FX_ID_PITCH_DOWN;
                        fx_x = *x;
                    }
                    ChannelCmd::FadeIn(x) => {
                        fx_id = FX_ID_FADE_IN;
                        fx_x = *x;
                    }
                    ChannelCmd::FadeOut(x) => {
                        fx_id = FX_ID_FADE_OUT;
                        fx_x = *x;
                    }
                    ChannelCmd::Tremble(x) => {
                        fx_id = FX_ID_TREMBLE;
                        fx_x = *x;
                    }
                    _ => {}
                }
            }

            let vol_out = if let Some(v) = explicit_vol {
                remembered_vol = v;
                muted = false;
                v
            } else if has_note_off {
                muted = true;
                0
            } else if has_note && muted {
                muted = false;
                remembered_vol
            } else {
                VOL_NO_CHANGE
            };

            let (mut fx_freq_x, mut fx_freq_y) = (0u16, 0u16);
            if fx_id == FX_ID_ARPEGGIO
                && let Some(name) = &cur_note_name
            {
                fx_freq_x = arp_offset_freq_inc(file, name, fx_x, sample_rate_hz).unwrap_or(0);
                if fx_y != ARP_NO_THIRD_NOTE {
                    fx_freq_y = arp_offset_freq_inc(file, name, fx_y, sample_rate_hz).unwrap_or(0);
                }
            } else if fx_id == FX_ID_PITCH_UP
                && let Some(name) = &cur_note_name
            {
                fx_freq_x = arp_offset_freq_inc(file, name, fx_x, sample_rate_hz).unwrap_or(cur_freq);
            } else if fx_id == FX_ID_PITCH_DOWN
                && let Some(name) = &cur_note_name
            {
                fx_freq_x =
                    pitch_down_offset_freq_inc(file, name, fx_x, sample_rate_hz).unwrap_or(cur_freq);
            }

            let base = 1 + ch * channel_stride;
            out[base + beat] = (cur_freq & 0xFF) as u8;
            out[base + beats + beat] = (cur_freq >> 8) as u8;
            out[base + beats * 2 + beat] = vol_out;
            out[base + beats * 3 + beat] = fx_id;
            out[base + beats * 4 + beat] = fx_x;
            out[base + beats * 5 + beat] = fx_y;
            out[base + beats * 6 + beat] = (fx_freq_x & 0xFF) as u8;
            out[base + beats * 7 + beat] = (fx_freq_x >> 8) as u8;
            out[base + beats * 8 + beat] = (fx_freq_y & 0xFF) as u8;
            out[base + beats * 9 + beat] = (fx_freq_y >> 8) as u8;
        }
    }

    // The SEQ lane carries one pattern-wide SequencerCmd per beat
    for beat in 0..beats {
        let seq_row = &pattern[0][beat];
        let (seq_cmd_type, seq_cmd_value, seq_cmd_value2) = match &seq_row.sqc {
            Some(SequencerCmd::Stop) => (SEQ_CMD_STOP, 0u8, 0u8),
            Some(SequencerCmd::Tempo(bpm)) => (SEQ_CMD_TEMPO, *bpm, 0u8),
            Some(SequencerCmd::FxSpeed(fx_speed)) => (SEQ_CMD_FX_SPEED, *fx_speed, 0u8),
            Some(SequencerCmd::FlowCount(count)) => (SEQ_CMD_FLOW_COUNT, *count, 0u8),
            Some(SequencerCmd::CountJump(pat, beat_target)) => {
                (SEQ_CMD_COUNT_JUMP, *pat, *beat_target)
            }
            Some(SequencerCmd::Jump(pat, beat_target)) => {
                (SEQ_CMD_JUMP, *pat, *beat_target)
            }
            None => (SEQ_CMD_NONE, 0u8, 0u8),
        };
        out[seq_cmd_base + beat] = seq_cmd_type;
        out[seq_cmd_base + beats + beat] = seq_cmd_value;
        out[seq_cmd_base + beats * 2 + beat] = seq_cmd_value2;
    }

    out
}

fn build_track_bin(bpm: u16, fx_speed: u8, baked: &[Vec<u8>]) -> Vec<u8> {
    let pattern_count = baked.len();
    let patterns_table_offset = TRACK_HEADER_SIZE;
    let mut pattern_offsets = Vec::with_capacity(pattern_count);
    let mut running = patterns_table_offset + pattern_count * 2;
    for data in baked {
        let pattern_size = data.len();
        pattern_offsets.push(running as u16);
        running += pattern_size;
    }

    let mut out = Vec::with_capacity(running);
    out.extend_from_slice(&bpm.to_le_bytes());
    out.extend_from_slice(&(fx_speed as u16).to_le_bytes());
    out.push(pattern_count as u8);
    for off in &pattern_offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    for data in baked {
        out.extend_from_slice(data);
    }
    out
}

pub fn export_all(
    file: &TrackerFile,
    bpm: u16,
    fx_speed: u8,
    stem: &str,
    export_dir: &Path,
) -> io::Result<()> {
    std::fs::create_dir_all(export_dir)?;

    let instruments_dir = export_dir.join("instruments");
    std::fs::create_dir_all(&instruments_dir)?;

    for i in 0..NUM_INSTRUMENTS {
        let path = instruments_dir.join(format!("instrument_{}.raw", i));
        std::fs::write(path, &file.instruments[i].waveform)?;
    }

    let baked: Vec<Vec<u8>> = (0..file.patterns.len())
        .map(|i| {
            bake_pattern(
                file,
                i,
                file.beats_for(i as u8) as usize,
                sample_rate_reg_to_hz(SAMPLE_RATE_REG),
            )
        })
        .collect();

    let track_bin = build_track_bin(bpm, fx_speed, &baked);
    std::fs::write(export_dir.join(format!("{}.bin", stem)), track_bin)?;

    Ok(())
}

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender, unbounded};
use gte_acp::{ARAM, AcpBus, audio_output::GameTankAudio};
use gte_w65c02s::W65C02S;
use indexmap::IndexMap;

use crate::file::NUM_INSTRUMENTS;
use crate::tracker::{ChannelCmd, NOISE_INSTRUMENT, Pattern, SequencerCmd, empty_pattern};

const FIRMWARE: &[u8; 4096] =
    include_bytes!("../../../../rom-template/gametank/audiofw/wavetable.bin");

const ROWS_PER_PATTERN: usize = 64;
const AUDIO_CHANNELS: usize = 7;
const CPU_FREQ: f64 = 3_579_545.0;

const INSTRUMENT_SLOT_COUNT: usize = 11;
const NOISE_SENTINEL_MODE0: u16 = 0xfffe;
const NOISE_SENTINEL_MODE1: u16 = 0xffff;

const WAVETABLE: [u16; INSTRUMENT_SLOT_COUNT] = [
    0x0300,
    0x0400,
    0x0500,
    0x0600,
    0x0700,
    0x0800,
    0x0900,
    0x0A00,
    0x0B00,
    0x0C00,
    NOISE_SENTINEL_MODE0,
];

fn resolve_wavetable(idx: usize, noise_mode: Option<u8>) -> u16 {
    if idx == NOISE_INSTRUMENT as usize {
        match noise_mode {
            Some(y) if y != 0 => NOISE_SENTINEL_MODE1,
            _ => NOISE_SENTINEL_MODE0,
        }
    } else {
        WAVETABLE[idx]
    }
}

pub enum PlayerCmd {
    Play(usize, usize),
    Pause,
    SetBpm(u16),
    SetFxSpeed(u8),
    UpdatePatterns(Vec<Pattern>, Vec<u8>),
    UpdateWaveform(usize, Box<[u8; 256]>),
    UpdateTuningNotes(IndexMap<String, f64>),
    SetChannelMuted(usize, bool),
}

struct PlayerInner {
    cmd_rx: Receiver<PlayerCmd>,
    current_row_out: Arc<AtomicUsize>,
    current_pattern_out: Arc<AtomicUsize>,
    is_playing_out: Arc<AtomicBool>,

    acp: W65C02S,
    acp_bus: AcpBus,
    acp_sample_rate: f64,
    acp_sample_rate_reg: u8,

    audio_out: GameTankAudio,
    output_sample_rate: f64,
    current_buffer: Option<[f32; 64]>,
    buffer_position: usize,
    patterns: Vec<Pattern>,
    pattern_beats_list: Vec<u8>,
    current_pattern_idx: usize,
    flow_count: u8,
    tuning_notes: IndexMap<String, f64>,

    playing: bool,
    current_row: usize,
    bpm: u16,
    fx_speed: u8,
    samples_per_beat: f64,
    samples_until_next_beat: f64,
    samples_per_tick: f64,
    samples_until_next_tick: f64,
    tick_count: u8,
    output_channels: usize,
    remembered_vol: [u8; AUDIO_CHANNELS],
    muted: [bool; AUDIO_CHANNELS],
    channel_muted: [bool; AUDIO_CHANNELS],
    base_freq: [u16; AUDIO_CHANNELS],
    current_instrument: [usize; AUDIO_CHANNELS],
    arp_active: [bool; AUDIO_CHANNELS],
    arp_x_freq: [u16; AUDIO_CHANNELS],
    arp_y_freq: [u16; AUDIO_CHANNELS],
    arp_step_count: [u8; AUDIO_CHANNELS],
    arp_step: [u8; AUDIO_CHANNELS],
    pitch_active: [bool; AUDIO_CHANNELS],
    pitch_target: [u16; AUDIO_CHANNELS],
    pitch_cur: [u16; AUDIO_CHANNELS],
    fade_active: [bool; AUDIO_CHANNELS],
    fade_target: [u8; AUDIO_CHANNELS],
    fade_cur: [u8; AUDIO_CHANNELS],
    vol_fx_hold: [u8; AUDIO_CHANNELS],
    vol_fx_counter: [u8; AUDIO_CHANNELS],
    tremble_active: [bool; AUDIO_CHANNELS],
    tremble_muted: [bool; AUDIO_CHANNELS],
    cur_note_name: [Option<String>; AUDIO_CHANNELS],
}

impl PlayerInner {
    fn new(
        cmd_rx: Receiver<PlayerCmd>,
        current_row_out: Arc<AtomicUsize>,
        current_pattern_out: Arc<AtomicUsize>,
        is_playing_out: Arc<AtomicBool>,
        output_sample_rate: f64,
        output_channels: usize,
        bpm: u16,
    ) -> Self {
        let sample_rate_reg = crate::sample_rate::SAMPLE_RATE_REG;
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr).copy_from_slice(FIRMWARE);
        }

        let mut acp = W65C02S::new();
        acp.reset();

        let acp_bus = AcpBus {
            irq_counter: (sample_rate_reg as i32) * 4,
            ..Default::default()
        };

        let acp_sample_rate = CPU_FREQ / sample_rate_reg as f64;
        let audio_out = GameTankAudio::new(acp_sample_rate, output_sample_rate);

        let samples_per_beat = output_sample_rate * 30.0 / (bpm.max(1) as f64);
        let fx_speed: u8 = 6;
        let samples_per_tick = samples_per_beat / fx_speed.max(1) as f64;

        Self {
            cmd_rx,
            current_row_out,
            current_pattern_out,
            is_playing_out,
            acp,
            acp_bus,
            acp_sample_rate,
            acp_sample_rate_reg: sample_rate_reg,
            audio_out,
            output_sample_rate,
            current_buffer: None,
            buffer_position: 0,
            patterns: vec![empty_pattern()],
            pattern_beats_list: vec![ROWS_PER_PATTERN as u8],
            current_pattern_idx: 0,
            flow_count: 0,
            tuning_notes: IndexMap::new(),
            playing: false,
            current_row: 0,
            bpm,
            fx_speed,
            samples_per_beat,
            samples_until_next_beat: samples_per_beat,
            samples_per_tick,
            samples_until_next_tick: samples_per_tick,
            tick_count: 0,
            output_channels,
            remembered_vol: [0; AUDIO_CHANNELS],
            muted: [false; AUDIO_CHANNELS],
            channel_muted: [false; AUDIO_CHANNELS],
            base_freq: [0; AUDIO_CHANNELS],
            current_instrument: std::array::from_fn(|ch| ch),
            arp_active: [false; AUDIO_CHANNELS],
            arp_x_freq: [0; AUDIO_CHANNELS],
            arp_y_freq: [0; AUDIO_CHANNELS],
            arp_step_count: [3; AUDIO_CHANNELS],
            arp_step: [0; AUDIO_CHANNELS],
            pitch_active: [false; AUDIO_CHANNELS],
            pitch_target: [0; AUDIO_CHANNELS],
            pitch_cur: [0; AUDIO_CHANNELS],
            fade_active: [false; AUDIO_CHANNELS],
            fade_target: [0; AUDIO_CHANNELS],
            fade_cur: [0; AUDIO_CHANNELS],
            vol_fx_hold: [0; AUDIO_CHANNELS],
            vol_fx_counter: [0; AUDIO_CHANNELS],
            tremble_active: [false; AUDIO_CHANNELS],
            tremble_muted: [false; AUDIO_CHANNELS],
            cur_note_name: std::array::from_fn(|_| None),
        }
    }

    fn recompute_tick_timing(&mut self) {
        self.samples_per_tick = self.samples_per_beat / self.fx_speed.max(1) as f64;
    }

    fn pattern_beats(&self, pattern_idx: usize) -> usize {
        self.pattern_beats_list
            .get(pattern_idx)
            .copied()
            .unwrap_or(ROWS_PER_PATTERN as u8)
            .clamp(1, ROWS_PER_PATTERN as u8) as usize
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                PlayerCmd::Play(pattern_idx, row) => {
                    self.playing = true;
                    self.current_pattern_idx =
                        pattern_idx.min(self.patterns.len().saturating_sub(1));
                    self.current_row = row;
                    self.flow_count = 0;
                    self.samples_until_next_beat = self.samples_per_beat;
                    self.samples_until_next_tick = self.samples_per_tick;
                    self.tick_count = 0;
                    self.rebuild_channel_state(self.current_pattern_idx, row);
                    for ch in 0..AUDIO_CHANNELS {
                        self.set_voice_volume(ch, if self.muted[ch] { 0 } else { self.remembered_vol[ch] });
                    }
                    self.trigger_row();
                    self.current_row_out.store(self.current_row, Ordering::Relaxed);
                    self.current_pattern_out
                        .store(self.current_pattern_idx, Ordering::Relaxed);
                    self.is_playing_out.store(true, Ordering::Relaxed);
                }
                PlayerCmd::Pause => {
                    self.playing = false;
                    self.is_playing_out.store(false, Ordering::Relaxed);
                    for ch in 0..AUDIO_CHANNELS {
                        self.set_voice_volume(ch, 0);
                    }
                    self.audio_out =
                        GameTankAudio::new(self.acp_sample_rate, self.output_sample_rate);
                    self.current_buffer = None;
                    self.buffer_position = 0;
                }
                PlayerCmd::SetBpm(bpm) => {
                    self.bpm = bpm;
                    self.samples_per_beat = self.output_sample_rate * 30.0 / (bpm.max(1) as f64);
                    self.recompute_tick_timing();
                }
                PlayerCmd::SetFxSpeed(fx_speed) => {
                    self.fx_speed = fx_speed;
                    self.recompute_tick_timing();
                }
                PlayerCmd::UpdatePatterns(patterns, beats_list) => {
                    if !patterns.is_empty() {
                        self.patterns = patterns;
                        self.pattern_beats_list = beats_list;
                        if self.current_pattern_idx >= self.patterns.len() {
                            self.current_pattern_idx = self.patterns.len() - 1;
                        }
                    }
                }
                PlayerCmd::UpdateWaveform(idx, wf) => {
                    if idx < NUM_INSTRUMENTS {
                        self.write_waveform(idx, &wf);
                    }
                }
                PlayerCmd::UpdateTuningNotes(notes) => {
                    self.tuning_notes = notes;
                }
                PlayerCmd::SetChannelMuted(ch, want_muted) => {
                    if ch < AUDIO_CHANNELS {
                        self.channel_muted[ch] = want_muted;
                        if want_muted {
                            self.set_voice_volume(ch, 0);
                        } else {
                            let vol = self.current_channel_volume(ch);
                            self.set_voice_volume(ch, vol);
                        }
                    }
                }
            }
        }
    }

    fn write_waveform(&mut self, idx: usize, waveform: &[u8; 256]) {
        let base = WAVETABLE[idx] as usize;
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (&mut (*aram_ptr))[base..base + 256].copy_from_slice(waveform);
        }
    }

    fn set_voice_frequency(&mut self, ch: usize, freq: u16) {
        let base = 0x0041 + (ch * 7);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 2] = (freq & 0xFF) as u8;
            (*aram_ptr)[base + 3] = (freq >> 8) as u8;
        }
    }

    fn set_voice_waveptr(&mut self, ch: usize, waveform_idx: usize, noise_mode: Option<u8>) {
        let base = 0x0041 + (ch * 7);
        let ptr = resolve_wavetable(waveform_idx, noise_mode);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 4] = (ptr & 0xFF) as u8;
            (*aram_ptr)[base + 5] = (ptr >> 8) as u8;
        }
    }

    fn set_voice_volume(&mut self, ch: usize, volume: u8) {
        let base = 0x0041 + (ch * 7);
        let volume = if self.channel_muted[ch] { 0 } else { volume.min(63) };
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 6] = volume;
        }
    }

    fn current_channel_volume(&self, ch: usize) -> u8 {
        if self.muted[ch] {
            0
        } else if self.fade_active[ch] {
            self.fade_cur[ch]
        } else if self.tremble_active[ch] && self.tremble_muted[ch] {
            0
        } else {
            self.remembered_vol[ch]
        }
    }

    fn rebuild_channel_state(&mut self, pattern_idx: usize, row: usize) {
        for ch in 0..AUDIO_CHANNELS {
            let mut vol = 0u8;
            let mut muted = true;
            let mut note_name: Option<String> = None;
            let mut base_freq: u16 = 0;
            let mut instrument = ch;
            let mut instrument_mode: Option<u8> = None;
            for r in 0..row {
                let beat = &self.patterns[pattern_idx][ch + 1][r];
                for cmd in &beat.cmd_list {
                    match cmd {
                        ChannelCmd::Volume(v) => {
                            vol = *v;
                            muted = false;
                        }
                        ChannelCmd::NoteOff => {
                            muted = true;
                        }
                        ChannelCmd::Note(name) => {
                            muted = false;
                            note_name = Some(name.clone());
                            if let Some(&freq_hz) = self.tuning_notes.get(name.as_str()) {
                                let freq_u32 =
                                    ((freq_hz / self.acp_sample_rate) * 65536.0).round() as u32;
                                base_freq = freq_u32.min(0xFFFF) as u16;
                            }
                        }
                        ChannelCmd::Instrument(idx, y) => {
                            instrument = *idx as usize;
                            instrument_mode = *y;
                        }
                        _ => {}
                    }
                }
            }
            self.remembered_vol[ch] = vol;
            self.muted[ch] = muted;
            self.cur_note_name[ch] = note_name;
            self.base_freq[ch] = base_freq;
            self.current_instrument[ch] = instrument;
            self.set_voice_waveptr(ch, instrument, instrument_mode);
            self.arp_active[ch] = false;
            self.pitch_active[ch] = false;
            self.fade_active[ch] = false;
        }

        for r in 0..row {
            if let Some(sqc) = &self.patterns[pattern_idx][0][r].sqc {
                match sqc {
                    SequencerCmd::Tempo(bpm) => self.bpm = *bpm as u16,
                    SequencerCmd::FxSpeed(fx_speed) => self.fx_speed = *fx_speed,
                    SequencerCmd::Stop
                    | SequencerCmd::FlowCount(_)
                    | SequencerCmd::CountJump(_, _) => {}
                    | SequencerCmd::Jump(_, _) => {}
                }
            }
        }
        self.samples_per_beat = self.output_sample_rate * 30.0 / (self.bpm.max(1) as f64);
        self.recompute_tick_timing();
    }

    // Process sequencer command and channel commands for the current row
    fn trigger_row(&mut self) {
        loop {
            let pattern_idx = self.current_pattern_idx;
            let row = self.current_row;

            if let Some(sqc) = self.patterns[pattern_idx][0][row].sqc.clone() {
                match sqc {
                    SequencerCmd::Stop => {
                        self.playing = false;
                        self.is_playing_out.store(false, Ordering::Relaxed);
                        for ch in 0..AUDIO_CHANNELS {
                            self.set_voice_volume(ch, 0);
                        }
                        return;
                    }
                    SequencerCmd::Tempo(bpm) => {
                        self.bpm = bpm as u16;
                        self.samples_per_beat =
                            self.output_sample_rate * 30.0 / (self.bpm.max(1) as f64);
                        self.recompute_tick_timing();
                    }
                    SequencerCmd::FxSpeed(fx_speed) => {
                        self.fx_speed = fx_speed;
                        self.recompute_tick_timing();
                    }
                    SequencerCmd::FlowCount(count) => {
                        self.flow_count = count;
                    }
                    SequencerCmd::CountJump(pat, beat) => {
                        if self.flow_count > 0 {
                            self.flow_count -= 1;
                            let target_pattern = (pat as usize).min(self.patterns.len() - 1);
                            let target_beat =
                                (beat as usize).min(self.pattern_beats(target_pattern) - 1);
                            self.current_pattern_idx = target_pattern;
                            self.current_row = target_beat;
                            self.current_pattern_out
                                .store(target_pattern, Ordering::Relaxed);
                            self.current_row_out.store(target_beat, Ordering::Relaxed);
                            continue;
                        }
                    }
                    SequencerCmd::Jump(pat, beat) => {
                        let target_pattern = (pat as usize).min(self.patterns.len() - 1);
                        let target_beat =
                            (beat as usize).min(self.pattern_beats(target_pattern) - 1);
                        self.current_pattern_idx = target_pattern;
                        self.current_row = target_beat;
                        self.current_pattern_out
                            .store(target_pattern, Ordering::Relaxed);
                        self.current_row_out.store(target_beat, Ordering::Relaxed);
                        continue;
                    }
                }
            }

            self.trigger_channels(pattern_idx, row);
            return;
        }
    }

    fn trigger_channels(&mut self, pattern_idx: usize, row: usize) {
        for ch in 0..AUDIO_CHANNELS {
            let beat = &self.patterns[pattern_idx][ch + 1][row];
            let maybe_note = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Note(s) => Some(s.clone()),
                _ => None,
            });
            let maybe_vol = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Volume(v) => Some(*v),
                _ => None,
            });
            let note_off = beat
                .cmd_list
                .iter()
                .any(|c| matches!(c, ChannelCmd::NoteOff));
            let maybe_arp = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Arpeggio(x, y) => Some((*x, *y)),
                _ => None,
            });
            let maybe_instrument = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Instrument(idx, y) => Some((*idx as usize, *y)),
                _ => None,
            });
            let maybe_pitch_up = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::PitchUp(x) => Some(*x),
                _ => None,
            });
            let maybe_pitch_down = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::PitchDown(x) => Some(*x),
                _ => None,
            });
            let maybe_fade_in = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::FadeIn(x) => Some(*x),
                _ => None,
            });
            let maybe_fade_out = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::FadeOut(x) => Some(*x),
                _ => None,
            });
            let maybe_tremble = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Tremble(x) => Some(*x),
                _ => None,
            });

            if let Some((idx, y)) = maybe_instrument {
                self.current_instrument[ch] = idx;
                self.set_voice_waveptr(ch, idx, y);
            }

            if let Some(v) = maybe_vol {
                self.remembered_vol[ch] = v;
                self.muted[ch] = false;
                self.set_voice_volume(ch, v);
            } else if note_off {
                self.muted[ch] = true;
                self.set_voice_volume(ch, 0);
            }

            if let Some(note_name) = &maybe_note {
                self.cur_note_name[ch] = Some(note_name.clone());
                if let Some(&freq_hz) = self.tuning_notes.get(note_name.as_str()) {
                    let freq_u32 = ((freq_hz / self.acp_sample_rate) * 65536.0).round() as u32;
                    let freq = freq_u32.min(0xFFFF) as u16;
                    self.base_freq[ch] = freq;
                    if maybe_vol.is_none() && self.muted[ch] {
                        self.muted[ch] = false;
                        self.set_voice_volume(ch, self.remembered_vol[ch]);
                    }
                }
            }

            let cur_effective_vol = if self.muted[ch] { 0 } else { self.remembered_vol[ch] };
            if let Some(x) = maybe_fade_in {
                self.fade_active[ch] = true;
                self.tremble_active[ch] = false;
                self.fade_target[ch] = cur_effective_vol;
                self.fade_cur[ch] = 0;
                self.vol_fx_hold[ch] = x;
                self.vol_fx_counter[ch] = x;
                self.set_voice_volume(ch, 0);
            } else if let Some(x) = maybe_fade_out {
                self.fade_active[ch] = true;
                self.tremble_active[ch] = false;
                self.fade_target[ch] = 0;
                self.fade_cur[ch] = cur_effective_vol;
                self.vol_fx_hold[ch] = x;
                self.vol_fx_counter[ch] = x;
                self.set_voice_volume(ch, cur_effective_vol);
            } else if let Some(x) = maybe_tremble {
                self.fade_active[ch] = false;
                self.tremble_active[ch] = true;
                self.vol_fx_hold[ch] = x;
                self.vol_fx_counter[ch] = x;
                self.tremble_muted[ch] = false;
                self.set_voice_volume(ch, cur_effective_vol);
            } else {
                self.fade_active[ch] = false;
                self.tremble_active[ch] = false;
                self.set_voice_volume(ch, cur_effective_vol);
            }

            if let Some((x, y)) = maybe_arp {
                let note_name = maybe_note.clone().or_else(|| self.cur_note_name[ch].clone());
                if let Some(note_name) = &note_name {
                    self.arp_x_freq[ch] = self
                        .arp_offset_freq(note_name, x)
                        .unwrap_or(self.base_freq[ch]);
                    self.arp_step_count[ch] = match y {
                        Some(y) => {
                            self.arp_y_freq[ch] = self
                                .arp_offset_freq(note_name, y)
                                .unwrap_or(self.base_freq[ch]);
                            3
                        }
                        None => 2,
                    };
                    self.arp_active[ch] = true;
                    self.pitch_active[ch] = false;
                } else {
                    self.arp_active[ch] = false;
                    self.pitch_active[ch] = false;
                    self.set_voice_frequency(ch, self.base_freq[ch]);
                }
            } else if let Some(x) = maybe_pitch_up {
                self.arp_active[ch] = false;
                let note_name = maybe_note.clone().or_else(|| self.cur_note_name[ch].clone());
                if let Some(note_name) = &note_name {
                    self.pitch_target[ch] = self
                        .arp_offset_freq(note_name, x)
                        .unwrap_or(self.base_freq[ch]);
                    self.pitch_cur[ch] = self.base_freq[ch];
                    self.pitch_active[ch] = true;
                    self.set_voice_frequency(ch, self.pitch_cur[ch]);
                } else {
                    self.pitch_active[ch] = false;
                    self.set_voice_frequency(ch, self.base_freq[ch]);
                }
            } else if let Some(x) = maybe_pitch_down {
                self.arp_active[ch] = false;
                let note_name = maybe_note.clone().or_else(|| self.cur_note_name[ch].clone());
                if let Some(note_name) = &note_name {
                    self.pitch_target[ch] = self
                        .pitch_down_offset_freq(note_name, x)
                        .unwrap_or(self.base_freq[ch]);
                    self.pitch_cur[ch] = self.base_freq[ch];
                    self.pitch_active[ch] = true;
                    self.set_voice_frequency(ch, self.pitch_cur[ch]);
                } else {
                    self.pitch_active[ch] = false;
                    self.set_voice_frequency(ch, self.base_freq[ch]);
                }
            } else {
                self.arp_active[ch] = false;
                self.pitch_active[ch] = false;
                self.set_voice_frequency(ch, self.base_freq[ch]);
            }
        }
    }

    fn arp_offset_freq(&self, note_name: &str, steps: u8) -> Option<u16> {
        let idx = self.tuning_notes.get_index_of(note_name)?;
        let target_idx = (idx + steps as usize).min(self.tuning_notes.len().saturating_sub(1));
        let (_, &hz) = self.tuning_notes.get_index(target_idx)?;
        let freq_u32 = ((hz / self.acp_sample_rate) * 65536.0).round() as u32;
        Some(freq_u32.min(0xFFFF) as u16)
    }

    fn pitch_down_offset_freq(&self, note_name: &str, steps: u8) -> Option<u16> {
        let idx = self.tuning_notes.get_index_of(note_name)?;
        let target_idx = idx.saturating_sub(steps as usize);
        let (_, &hz) = self.tuning_notes.get_index(target_idx)?;
        let freq_u32 = ((hz / self.acp_sample_rate) * 65536.0).round() as u32;
        Some(freq_u32.min(0xFFFF) as u16)
    }

    fn advance_tick(&mut self) {
        for ch in 0..AUDIO_CHANNELS {
            if self.arp_active[ch] {
                let freq = match self.arp_step[ch] {
                    0 => self.base_freq[ch],
                    1 => self.arp_x_freq[ch],
                    _ => self.arp_y_freq[ch],
                };
                self.set_voice_frequency(ch, freq);
                self.arp_step[ch] = if self.arp_step[ch] + 1 >= self.arp_step_count[ch] {
                    0
                } else {
                    self.arp_step[ch] + 1
                };
            } else if self.pitch_active[ch] {
                let speed = (self.fx_speed as u16) * 4;
                let cur = self.pitch_cur[ch];
                let target = self.pitch_target[ch];
                let next = if cur < target {
                    let stepped = cur.saturating_add(speed);
                    if stepped >= target {
                        self.pitch_active[ch] = false;
                        target
                    } else {
                        stepped
                    }
                } else if cur > target {
                    let stepped = cur.saturating_sub(speed);
                    if stepped <= target {
                        self.pitch_active[ch] = false;
                        target
                    } else {
                        stepped
                    }
                } else {
                    self.pitch_active[ch] = false;
                    target
                };
                self.pitch_cur[ch] = next;
                self.set_voice_frequency(ch, next);
            }

            if self.fade_active[ch] {
                if self.vol_fx_counter[ch] == 0 {
                    let cur = self.fade_cur[ch];
                    let target = self.fade_target[ch];
                    let next = if cur < target {
                        cur + 1
                    } else if cur > target {
                        cur - 1
                    } else {
                        cur
                    };
                    self.fade_cur[ch] = next;
                    self.set_voice_volume(ch, next);
                    if next == target {
                        self.fade_active[ch] = false;
                    }
                    self.vol_fx_counter[ch] = self.vol_fx_hold[ch];
                } else {
                    self.vol_fx_counter[ch] -= 1;
                }
            } else if self.tremble_active[ch] {
                if self.vol_fx_counter[ch] == 0 {
                    self.tremble_muted[ch] = !self.tremble_muted[ch];
                    let vol = if self.tremble_muted[ch] || self.muted[ch] {
                        0
                    } else {
                        self.remembered_vol[ch]
                    };
                    self.set_voice_volume(ch, vol);
                    self.vol_fx_counter[ch] = self.vol_fx_hold[ch];
                } else {
                    self.vol_fx_counter[ch] -= 1;
                }
            }
        }
    }

    fn run_acp_until_sample(&mut self) -> bool {
        let cycles_per_sample = (self.acp_sample_rate_reg as i32) * 4;
        let max_cycles = cycles_per_sample * 8;
        let mut cycles_run = 0;

        loop {
            let acp_cycles = self.acp.step(&mut self.acp_bus);
            cycles_run += acp_cycles;
            self.acp_bus.irq_counter -= acp_cycles;

            self.acp.set_irq(false);
            self.acp.set_nmi(false);

            if self.acp_bus.irq_counter <= 0 {
                self.acp_bus.irq_counter += cycles_per_sample;
                self.acp.set_irq(true);

                let sample_u8 = self.acp_bus.sample;
                let _ = self.audio_out.producer.push(sample_u8);
                return true;
            }

            if cycles_run >= max_cycles {
                return false;
            }
        }
    }

    fn fill_output(&mut self, data: &mut [f32]) {
        self.process_commands();

        // Prevent buffer underrun
        if !self.playing {
            data.fill(0.0);
            return;
        }

        let out_ch = self.output_channels;
        let frame_count = data.len() / out_ch;

        for frame in 0..frame_count {
            self.samples_until_next_tick -= 1.0;
            if self.samples_until_next_tick <= 0.0 {
                self.samples_until_next_tick += self.samples_per_tick.max(1.0);
                self.advance_tick();
            }

            self.samples_until_next_beat -= 1.0;
            if self.samples_until_next_beat <= 0.0 {
                self.samples_until_next_beat += self.samples_per_beat;
                let beats = self.pattern_beats(self.current_pattern_idx);
                self.current_row = (self.current_row + 1) % beats.max(1);
                self.samples_until_next_tick = self.samples_per_tick.max(1.0);
                self.tick_count = 0;
                self.trigger_row();
                self.current_row_out
                    .store(self.current_row, Ordering::Relaxed);
                self.current_pattern_out
                    .store(self.current_pattern_idx, Ordering::Relaxed);
            }

            if self.current_buffer.is_none() || self.buffer_position >= 64 {
                self.current_buffer = None;
                self.buffer_position = 0;

                while self.audio_out.output_buffer.slots() == 0 {
                    for _ in 0..128 {
                        self.run_acp_until_sample();
                    }
                    self.audio_out.convert_to_output_buffers();
                }

                if let Ok(buffer) = self.audio_out.output_buffer.pop() {
                    let mut arr = [0.0f32; 64];
                    for (i, &val) in buffer.iter().enumerate() {
                        arr[i] = val;
                    }
                    self.current_buffer = Some(arr);
                }
            }

            let sample = match self.current_buffer {
                Some(ref buffer) if self.buffer_position < 64 => {
                    let s = buffer[self.buffer_position];
                    self.buffer_position += 1;
                    s
                }
                _ => 0.0,
            };

            for ch in 0..out_ch {
                data[frame * out_ch + ch] = sample;
            }
        }
    }
}

pub struct Player {
    cmd_tx: Sender<PlayerCmd>,
    current_row: Arc<AtomicUsize>,
    current_pattern: Arc<AtomicUsize>,
    is_playing: Arc<AtomicBool>,
    _stream: cpal::Stream,
}

impl Player {
    pub fn new(bpm: u16) -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;

        let output_sample_rate = config.sample_rate().0 as f64;
        let output_channels = config.channels() as usize;

        let (cmd_tx, cmd_rx) = unbounded::<PlayerCmd>();
        let current_row = Arc::new(AtomicUsize::new(0));
        let current_pattern = Arc::new(AtomicUsize::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));

        let mut inner = PlayerInner::new(
            cmd_rx,
            current_row.clone(),
            current_pattern.clone(),
            is_playing.clone(),
            output_sample_rate,
            output_channels,
            bpm,
        );

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    inner.fill_output(data);
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .ok()?;

        stream.play().ok()?;

        Some(Player {
            cmd_tx,
            current_row,
            current_pattern,
            is_playing,
            _stream: stream,
        })
    }

    pub fn play(&self, pattern_idx: usize, row: usize) {
        let _ = self.cmd_tx.send(PlayerCmd::Play(pattern_idx, row));
    }

    pub fn pause(&self) {
        let _ = self.cmd_tx.send(PlayerCmd::Pause);
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn current_row(&self) -> usize {
        self.current_row.load(Ordering::Relaxed)
    }

    pub fn current_pattern(&self) -> usize {
        self.current_pattern.load(Ordering::Relaxed)
    }

    pub fn set_bpm(&self, bpm: u16) {
        let _ = self.cmd_tx.send(PlayerCmd::SetBpm(bpm));
    }

    pub fn set_fx_speed(&self, fx_speed: u8) {
        let _ = self.cmd_tx.send(PlayerCmd::SetFxSpeed(fx_speed));
    }

    pub fn update_patterns(&self, patterns: Vec<Pattern>, beats_list: Vec<u8>) {
        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdatePatterns(patterns, beats_list));
    }

    pub fn update_waveform(&self, idx: usize, waveform: [u8; 256]) {
        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdateWaveform(idx, Box::new(waveform)));
    }

    pub fn update_tuning_notes(&self, notes: IndexMap<String, f64>) {
        let _ = self.cmd_tx.send(PlayerCmd::UpdateTuningNotes(notes));
    }

    pub fn set_channel_muted(&self, ch: usize, muted: bool) {
        let _ = self.cmd_tx.send(PlayerCmd::SetChannelMuted(ch, muted));
    }
}

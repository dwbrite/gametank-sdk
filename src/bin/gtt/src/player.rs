use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender, unbounded};
use gte_acp::{ARAM, AcpBus, audio_output::GameTankAudio};
use gte_w65c02s::{System, W65C02S};
use indexmap::IndexMap;

use crate::tracker::{ChannelCmd, Pattern, empty_pattern};

const FIRMWARE: &[u8; 4096] =
    include_bytes!("../../../../rom-template/gametank/audiofw/wavetable-8ch.bin");

const ROWS_PER_PATTERN: usize = 64;
const AUDIO_CHANNELS: usize = 8;
const CPU_FREQ: f64 = 3_579_545.0;

pub enum PlayerCmd {
    Play(usize),
    Pause,
    SetBpm(f64),
    UpdatePattern(Box<Pattern>),
    UpdateWaveform(usize, Box<[u8; 256]>),
    UpdateTuningNotes(IndexMap<String, f64>),
    SetSampleRate(u8),
}

struct PlayerInner {
    cmd_rx: Receiver<PlayerCmd>,
    current_row_out: Arc<AtomicUsize>,
    is_playing_out: Arc<AtomicBool>,

    acp: W65C02S,
    acp_bus: AcpBus,
    acp_sample_rate: f64,
    acp_sample_rate_reg: u8,

    audio_out: GameTankAudio,
    output_sample_rate: f64,
    current_buffer: Option<[f32; 64]>,
    buffer_position: usize,
    pattern: Box<Pattern>,
    tuning_notes: IndexMap<String, f64>,

    playing: bool,
    current_row: usize,
    samples_per_beat: f64,
    samples_until_next_beat: f64,
    output_channels: usize,
}

impl PlayerInner {
    fn new(
        cmd_rx: Receiver<PlayerCmd>,
        current_row_out: Arc<AtomicUsize>,
        is_playing_out: Arc<AtomicBool>,
        output_sample_rate: f64,
        output_channels: usize,
        bpm: f64,
        sample_rate_reg: u8,
    ) -> Self {
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr).copy_from_slice(FIRMWARE);
        }

        let mut acp = W65C02S::new();
        acp.reset();

        let mut acp_bus = AcpBus::default();
        acp_bus.irq_counter = (sample_rate_reg as i32) * 4;

        let acp_sample_rate = CPU_FREQ / sample_rate_reg as f64;
        let audio_out = GameTankAudio::new(acp_sample_rate, output_sample_rate);

        let samples_per_beat = output_sample_rate * 60.0 / bpm.max(1.0);

        Self {
            cmd_rx,
            current_row_out,
            is_playing_out,
            acp,
            acp_bus,
            acp_sample_rate,
            acp_sample_rate_reg: sample_rate_reg,
            audio_out,
            output_sample_rate,
            current_buffer: None,
            buffer_position: 0,
            pattern: Box::new(empty_pattern()),
            tuning_notes: IndexMap::new(),
            playing: false,
            current_row: 0,
            samples_per_beat,
            samples_until_next_beat: samples_per_beat,
            output_channels,
        }
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                PlayerCmd::Play(row) => {
                    self.playing = true;
                    self.current_row = row;
                    self.samples_until_next_beat = self.samples_per_beat;
                    self.trigger_row();
                    self.current_row_out.store(row, Ordering::Relaxed);
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
                    self.samples_per_beat = self.output_sample_rate * 60.0 / bpm.max(1.0);
                }
                PlayerCmd::UpdatePattern(pat) => {
                    self.pattern = pat;
                }
                PlayerCmd::UpdateWaveform(idx, wf) => {
                    if idx < AUDIO_CHANNELS {
                        self.write_waveform(idx, &wf);
                    }
                }
                PlayerCmd::UpdateTuningNotes(notes) => {
                    self.tuning_notes = notes;
                }
                PlayerCmd::SetSampleRate(reg) => {
                    self.acp_sample_rate_reg = reg;
                    self.acp_sample_rate = CPU_FREQ / reg as f64;
                    self.acp_bus.irq_counter = (reg as i32) * 4;
                    self.audio_out =
                        GameTankAudio::new(self.acp_sample_rate, self.output_sample_rate);
                }
            }
        }
    }

    fn write_waveform(&mut self, idx: usize, waveform: &[u8; 256]) {
        let base = 0x0400 + (idx * 0x100);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            for i in 0..256 {
                (*aram_ptr)[base + i] = waveform[i];
            }
        }
    }

    fn set_voice_frequency(&mut self, ch: usize, freq: u16) {
        let base = 0x0041 + (ch * 7);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 2] = (freq & 0xFF) as u8;
            (*aram_ptr)[base + 3] = (freq >> 8) as u8;
            (*aram_ptr)[base] = 0;
            (*aram_ptr)[base + 1] = 0;
        }
    }

    fn set_voice_waveptr(&mut self, ch: usize, waveform_idx: usize) {
        let base = 0x0041 + (ch * 7);
        let ptr = 0x0400 + (waveform_idx * 0x100);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 4] = (ptr & 0xFF) as u8;
            (*aram_ptr)[base + 5] = (ptr >> 8) as u8;
        }
    }

    fn set_voice_volume(&mut self, ch: usize, volume: u8) {
        let base = 0x0041 + (ch * 7);
        unsafe {
            let aram_ptr = std::ptr::addr_of_mut!(ARAM);
            (*aram_ptr)[base + 6] = volume.min(63);
        }
    }

    fn trigger_row(&mut self) {
        let row = self.current_row;
        for ch in 0..AUDIO_CHANNELS {
            let beat = &self.pattern[ch + 1][row];
            let maybe_note = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Note(s) => Some(s.as_str()),
                _ => None,
            });
            let maybe_vol = beat.cmd_list.iter().find_map(|c| match c {
                ChannelCmd::Volume(v) => Some(*v),
                _ => None,
            });
            if let Some(note_name) = maybe_note {
                if let Some(&freq_hz) = self.tuning_notes.get(note_name) {
                    let freq_u32 = ((freq_hz / self.acp_sample_rate) * 65536.0).round() as u32;
                    let freq = freq_u32.min(0xFFFF) as u16;
                    self.set_voice_frequency(ch, freq);
                    self.set_voice_waveptr(ch, ch);
                    let vol = maybe_vol.unwrap_or(32);
                    self.set_voice_volume(ch, vol);
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
            self.samples_until_next_beat -= 1.0;
            if self.samples_until_next_beat <= 0.0 {
                self.samples_until_next_beat += self.samples_per_beat;
                self.current_row = (self.current_row + 1) % ROWS_PER_PATTERN;
                self.trigger_row();
                self.current_row_out
                    .store(self.current_row, Ordering::Relaxed);
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
    is_playing: Arc<AtomicBool>,
    _stream: cpal::Stream,
}

impl Player {
    pub fn new(bpm: f64, sample_rate_reg: u8) -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;

        let output_sample_rate = config.sample_rate().0 as f64;
        let output_channels = config.channels() as usize;

        let (cmd_tx, cmd_rx) = unbounded::<PlayerCmd>();
        let current_row = Arc::new(AtomicUsize::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));

        let mut inner = PlayerInner::new(
            cmd_rx,
            current_row.clone(),
            is_playing.clone(),
            output_sample_rate,
            output_channels,
            bpm,
            sample_rate_reg,
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
            is_playing,
            _stream: stream,
        })
    }

    pub fn play(&self, row: usize) {
        let _ = self.cmd_tx.send(PlayerCmd::Play(row));
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

    pub fn set_bpm(&self, bpm: f64) {
        let _ = self.cmd_tx.send(PlayerCmd::SetBpm(bpm));
    }

    pub fn update_pattern(&self, pattern: Pattern) {
        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdatePattern(Box::new(pattern)));
    }

    pub fn update_waveform(&self, idx: usize, waveform: [u8; 256]) {
        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdateWaveform(idx, Box::new(waveform)));
    }

    pub fn update_tuning_notes(&self, notes: IndexMap<String, f64>) {
        let _ = self.cmd_tx.send(PlayerCmd::UpdateTuningNotes(notes));
    }

    pub fn set_sample_rate(&self, reg: u8) {
        let _ = self.cmd_tx.send(PlayerCmd::SetSampleRate(reg));
    }
}

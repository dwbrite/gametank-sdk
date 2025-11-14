use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use gte_w65c02s::{System, W65C02S};
use log::{debug, error, info, warn};
use gte_w65c02s::State::AwaitingInterrupt;
use core::fmt::{Debug, Formatter};
use bytemuck::bytes_of;
use heapless::{FnvIndexMap};
use rtrb::PushError;
use gte_acp::audio_output::GameTankAudio;
use crate::blitter::Blitter;
use crate::cartridges::CartridgeType;
use crate::emulator::PlayState::{Paused, Playing, WasmInit};
use crate::gametank_bus::{CpuBus};
use gte_acp::AcpBus;
use crate::inputs::{ControllerButton, InputCommand, KeyState};
use crate::inputs::ControllerButton::{Down, Left, Right, Start, Up, A, B, C};
use crate::inputs::InputCommand::{Controller1, Controller2, HardReset, PlayPause, SoftReset};
use crate::inputs::KeyState::JustReleased;

pub const WIDTH: u32 = 128;
pub const HEIGHT: u32 = 128;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlayState {
    WasmInit,
    Paused,
    Playing,
}

pub trait TimeDaemon {
    fn get_now_ms(&self) -> f64;
}

pub struct Emulator<Clock: TimeDaemon> {
    pub cpu_bus: CpuBus,
    pub acp_bus: AcpBus,
    pub cpu: W65C02S,
    pub acp: W65C02S,

    pub blitter: Blitter,

    pub clock_cycles_to_vblank: i32,

    pub last_emu_tick: f64,
    pub cpu_ns_per_cycle: f64,
    pub cpu_frequency_hz: f64,
    pub last_render_time: f64,
    pub audio_out: Option<GameTankAudio>,
    pub target_sample_rate: f64,
    pub play_state: PlayState,
    pub wait_counter: u64,

    pub input_state: FnvIndexMap<InputCommand, KeyState, 32>, // capacity of 32 entries

    pub clock: Clock,
}

impl <Clock: TimeDaemon> Emulator<Clock> {
    pub fn load_rom(&mut self, bytes: &[u8]) {
        warn!("loading new rom from memory, size: {}", bytes.len());
        self.cpu_bus.cartridge = CartridgeType::from_slice(bytes);
        warn!(" - cartridge loaded from memory");
        self.cpu.reset();
        warn!(" - cpu reset");
        self.acp.reset();
        warn!(" - acp reset");
        self.blitter.clear_irq_trigger();
        warn!(" - blitter irq cleared");
    }
}

impl <Clock: TimeDaemon> Debug for Emulator<Clock> {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        f.debug_struct("Emulator")
            .field("cpu_bus", &self.cpu_bus)
            .field("acp_bus", &self.acp_bus)
            .field("cpu", &self.cpu)
            .field("acp", &self.acp)
            .field("blitter", &self.blitter)
            .field("clock_cycles_to_vblank", &self.clock_cycles_to_vblank)
            .field("last_emu_tick", &self.last_emu_tick);

        Ok(())
    }}


impl <Clock: TimeDaemon> Emulator<Clock> {
    pub fn wasm_init(&mut self) {
        if self.play_state == WasmInit {
            self.play_state = Playing;
            self.last_emu_tick = self.clock.get_now_ms();
            self.last_render_time = self.clock.get_now_ms();
        }
    }

    pub fn init(clock: Clock, target_sample_rate: f64) -> Self {
        let play_state = WasmInit;

        let mut bus = CpuBus::default();
        let mut cpu = W65C02S::new();
        cpu.step(&mut bus); // take one initial step, to get through the reset vector
        let acp = W65C02S::new();

        let blitter = Blitter::default();

        let last_cpu_tick_ms = clock.get_now_ms();
        let cpu_frequency_hz = 3_579_545.0; // Precise frequency
        let cpu_ns_per_cycle = 1_000_000_000.0 / cpu_frequency_hz; // Nanoseconds per cycle

        let last_render_time = last_cpu_tick_ms;

        Emulator {
            play_state,
            cpu_bus: bus,
            acp_bus: AcpBus::default(),
            cpu,
            acp,
            blitter,

            clock_cycles_to_vblank: 59659,
            last_emu_tick: last_cpu_tick_ms,
            cpu_frequency_hz,
            cpu_ns_per_cycle,
            last_render_time,
            audio_out: None,
            target_sample_rate,
            wait_counter: 0,
            input_state: Default::default(),
            clock,
        }
    }

    pub fn process_cycles(&mut self, is_web: bool) {
        self.process_inputs();

        if self.play_state != Playing {
            return
        }

        let now_ms = self.clock.get_now_ms();
        let mut elapsed_ms = now_ms - self.last_emu_tick;

        if elapsed_ms > 33.0 {
            warn!("emulator took more than 33ms to process cycles");
            elapsed_ms = 16.667;
        }

        let elapsed_ns = elapsed_ms * 1000000.0;
        let mut remaining_cycles: i32 = (elapsed_ns / self.cpu_ns_per_cycle) as i32;

        let mut acp_cycle_accumulator = 0;

        while remaining_cycles > 0 {
            if self.cpu.get_state() == AwaitingInterrupt {
                self.wait_counter += 1;
                // get cpu's current asm code
            } else if self.wait_counter > 0 {
                debug!("waited {} cycles", self.wait_counter);
                self.wait_counter = 0;
            }

            let cpu_cycles = self.cpu.step(&mut self.cpu_bus);

            remaining_cycles -= cpu_cycles;

            acp_cycle_accumulator += cpu_cycles * 4;

            // pass aram to acp
            if self.cpu_bus.system_control.acp_enabled() {
                self.run_acp(&mut acp_cycle_accumulator);
            }

            // blit
            for _ in 0..cpu_cycles {
                self.blitter.cycle(&mut self.cpu_bus);
            }
            // TODO: instant blit option

            let blit_irq = self.blitter.irq_trigger;
            if blit_irq {
                debug!("blit irq");
            }
            self.cpu.set_irq(blit_irq);

            self.clock_cycles_to_vblank -= cpu_cycles;
            if self.clock_cycles_to_vblank <= 0 {
                self.vblank();
            }
        }

        self.last_emu_tick = now_ms;

        if !is_web && (now_ms - self.last_render_time) >= 16.67 {
            debug!("time since last render: {}", now_ms - self.last_render_time);
            self.last_render_time = now_ms;
        }
    }

    fn run_acp(&mut self, acp_cycle_accumulator: &mut i32) {
        if self.cpu_bus.system_control.clear_acp_reset() {
            self.acp.reset();
        }

        if self.cpu_bus.system_control.clear_acp_nmi() {
            self.acp.set_nmi(true);
        }

        while *acp_cycle_accumulator > 0 {
            let acp_cycles = self.acp.step(&mut self.acp_bus);
            *acp_cycle_accumulator -= acp_cycles;
            self.acp_bus.irq_counter -= acp_cycles;

            // clear stuff ig
            self.acp.set_irq(false);
            self.acp.set_nmi(false);

            if self.acp_bus.irq_counter <= 0 {
                self.acp_bus.irq_counter = self.cpu_bus.system_control.sample_rate() as i32 * 4;
                self.acp.set_irq(true);

                let sample_rate = self.cpu_frequency_hz / self.cpu_bus.system_control.sample_rate() as f64;
                // if audio_out is none or mismatched sample rate
                if self.audio_out.as_ref().map_or(true, |gta| gta.sample_rate != sample_rate) {
                    warn!("recreated audio stream with new sample rate: {:.3}Hz ({})", sample_rate, self.cpu_bus.system_control.sample_rate());
                    self.audio_out = Some(GameTankAudio::new(sample_rate, self.target_sample_rate));
                }

                if let Some(audio) = &mut self.audio_out {
                    let next_sample_u8 = self.acp_bus.sample;
                    if let Err(e) = audio.producer.push(next_sample_u8) {
                        error!("not enough slots in audio producer: {e}");
                    }
                }

                if let Some(audio) = &mut self.audio_out {
                    audio.convert_to_output_buffers();
                    // audio.process_audio();
                }
            }
        }
    }

    fn vblank(&mut self) {
        self.clock_cycles_to_vblank += 59659;

        if self.cpu_bus.vblank_nmi_enabled() {
            self.cpu.set_nmi(true);
            debug!("vblanked");
        }
    }

    pub fn set_input_state(&mut self, input_command: InputCommand, state: KeyState) {
        self.input_state.insert(input_command, state).expect("shit's full dog ://");
    }

    fn process_inputs(&mut self) {
        let keys: Vec<_> = self.input_state.keys().cloned().collect();  // Clone keys to avoid borrowing conflicts

        if keys.len() > 0 && self.play_state == WasmInit {
            self.play_state = Playing;
        }

        for key in &keys {
            match key {
                Controller1(button) => { self.set_gamepad_input(0, &key, &button); }
                Controller2(button) => { self.set_gamepad_input(1, &key, &button); }
                PlayPause => {
                    if self.input_state[key] == JustReleased {
                        match self.play_state {
                            Paused => { self.play_state = Playing; }
                            Playing => { self.play_state = Paused; }
                            WasmInit => { self.play_state = Playing; }
                        }
                    }
                }
                SoftReset => {
                    self.cpu.reset();
                }
                HardReset => {
                    // hard reset reinitializes memory/cpus
                    let cart = self.cpu_bus.cartridge.clone();
                    self.cpu_bus = CpuBus::default();
                    self.cpu_bus.cartridge = cart;
                    self.cpu = W65C02S::new();
                    self.cpu.step(&mut self.cpu_bus); // take one initial step, to get through the reset vector
                    self.acp = W65C02S::new();
                    self.blitter = Blitter::default();
                }
            }
            self.input_state.insert(*key, self.input_state[key].update()).expect("shit's full dog ://");
        }
    }
    fn set_gamepad_input(&mut self, gamepad: usize, key: &InputCommand, button: &ControllerButton) {
        let gamepad = &mut self.cpu_bus.system_control.gamepads[gamepad];
        match button {
            Up =>     { gamepad.up    = self.input_state[&key].is_pressed(); }
            Down =>   { gamepad.down  = self.input_state[&key].is_pressed(); }
            Left =>   { gamepad.left  = self.input_state[&key].is_pressed(); }
            Right =>  { gamepad.right = self.input_state[&key].is_pressed(); }
            B =>      { gamepad.b     = self.input_state[&key].is_pressed(); }
            A =>      { gamepad.a     = self.input_state[&key].is_pressed(); }
            Start =>  { gamepad.start = self.input_state[&key].is_pressed(); }
            C =>      { gamepad.c     = self.input_state[&key].is_pressed(); }
        }
    }
}
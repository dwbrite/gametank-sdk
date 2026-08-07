#![allow(unused)]

use std::collections::HashMap;

#[macro_use]
use libretro_rs::prelude::*;
use libretro_rs::retro::env::SetEnvironment;

use std::ffi::c_uint;
use std::time::Instant;
use gte_core::color_map::COLOR_MAP;
use gte_core::emulator::{Emulator, PlayState, TimeDaemon};
use gte_core::inputs::{ControllerButton, InputCommand, KeyState};
use gte_core::inputs::InputCommand::{Controller1, Controller2};
use gte_core::inputs::KeyState::{JustPressed, JustReleased};
use libretro_rs::prelude::env::{GetAvInfo, Init, Reset, Run, UnloadGame};

struct CoreEmulator {
    emu: Emulator<InstantClock>,
    rendering_mode: Option<SoftwareRenderEnabled>,
    input_bindings: HashMap<(c_uint, JoypadButton), InputCommand>,
    pixel_format: Option<ActiveFormat<ORGB1555>>,
    framebuffer: FrameBufferThing,
}

struct FrameBufferThing {
    video_frame: Vec<u8>
}

struct InstantClock {
    instant: Instant,
}

impl TimeDaemon for InstantClock {
    fn get_now_ms(&self) -> f64 {
        self.instant.elapsed().as_millis() as f64
    }
}

impl Default for CoreEmulator {
    fn default() -> Self {
        let clock = InstantClock { instant: Instant::now() };

        let mut input_bindings = HashMap::new();

        input_bindings.insert((0, JoypadButton::Start), Controller1(ControllerButton::Start));
        input_bindings.insert((0, JoypadButton::Up), Controller1(ControllerButton::Up));
        input_bindings.insert((0, JoypadButton::Down), Controller1(ControllerButton::Down));
        input_bindings.insert((0, JoypadButton::Left), Controller1(ControllerButton::Left));
        input_bindings.insert((0, JoypadButton::Right), Controller1(ControllerButton::Right));
        input_bindings.insert((0, JoypadButton::A), Controller1(ControllerButton::A));
        input_bindings.insert((0, JoypadButton::B), Controller1(ControllerButton::B));
        input_bindings.insert((0, JoypadButton::Y), Controller1(ControllerButton::C));

        input_bindings.insert((1, JoypadButton::Start), Controller2(ControllerButton::Start));
        input_bindings.insert((1, JoypadButton::Up), Controller2(ControllerButton::Up));
        input_bindings.insert((1, JoypadButton::Down), Controller2(ControllerButton::Down));
        input_bindings.insert((1, JoypadButton::Left), Controller2(ControllerButton::Left));
        input_bindings.insert((1, JoypadButton::Right), Controller2(ControllerButton::Right));
        input_bindings.insert((1, JoypadButton::A), Controller2(ControllerButton::A));
        input_bindings.insert((1, JoypadButton::B), Controller2(ControllerButton::B));
        input_bindings.insert((1, JoypadButton::Y), Controller2(ControllerButton::C));

        Self {
            emu: Emulator::init(clock, 44100.0),
            input_bindings,
            rendering_mode: None,
            pixel_format: None,
            framebuffer: FrameBufferThing { video_frame: vec![] },
        }
    }
}

pub fn buffer_to_color_image(framebuffer: &[u8; 128*128]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(128 * 128 * 2);

    for &index in framebuffer.iter() {
        let (r, g, b, _) = COLOR_MAP[index as usize];

        // Convert 8-bit channels → 5 bits each, ignore alpha.
        let r5 = (r >> 3) as u16;
        let g5 = (g >> 3) as u16;
        let b5 = (b >> 3) as u16;

        // Pack into 0RGB1555 (bit15=0)
        let packed = (r5 << 10) | (g5 << 5) | b5;

        pixels.push((packed & 0xFF) as u8);
        pixels.push((packed >> 8) as u8);
    }
    
    pixels
}

impl<'a> Core<'a> for CoreEmulator {
    type Init = Self;

    fn get_system_info() -> SystemInfo {
        SystemInfo::new(
            c_utf8!("GameTank Rust!"),
            c_utf8!("0.1.2"),
            ext!["zip", "gtr", "gtrs", "bin", "rom", "hex"],
        )
    }

    fn init(env: &mut impl Init) -> Self::Init {        
        env.set_support_no_game(true);
        Self::default()
    }

    fn load_game<E: env::LoadGame>(
        game: &GameInfo,
        args: LoadGameExtraArgs<'a, '_, E, Self::Init>,
    ) -> Result<Self, CoreError> {
        let LoadGameExtraArgs { env, pixel_format, rendering_mode, .. } = args;
        let pixel_format = env.set_pixel_format_0rgb1555(pixel_format)?;
        let game_data = unsafe { game.as_data_unchecked() };

        let game_slice = game_data.data();

        let mut core = Self::default();
        core.emu.load_rom(game_slice);
        // core.game_data = Some(game_data);
        core.emu.play_state = PlayState::Playing;
        core.rendering_mode = Some(rendering_mode);
        core.pixel_format = Some(pixel_format);

        Ok(core)
    }

    fn get_system_av_info(&self, env: &mut impl GetAvInfo) -> SystemAVInfo {
        // default timing is 60FPS, 44.1KHz
        SystemAVInfo::default_timings(GameGeometry::fixed(128, 128))
    }

    fn run(&mut self, env: &mut impl Run, callbacks: &mut impl Callbacks) -> InputsPolled {
        let inputs_polled = callbacks.poll_inputs();
        // update emulator inputs
        for ((port, button), command) in &self.input_bindings {
            if let Some(ks) = self.emu.input_state.get(&command) {
                self.emu.set_input_state(*command, ks.update_state(callbacks.is_joypad_button_pressed(DevicePort::new(*port), *button)))
            } else {
                self.emu.set_input_state(*command, KeyState::new(callbacks.is_joypad_button_pressed(DevicePort::new(*port), *button)))
            }
        }
        
        self.emu.process_cycles(false);
        if let Some(ref mut audio_out) = &mut self.emu.audio_out {
            let mut audio_samples = Vec::with_capacity(4096);
            while !audio_out.output_buffer.is_empty() {
                if let Ok(buffer) = audio_out.output_buffer.pop() {
                    // is this going to kill perf???
                    for sample in buffer.iter() {
                        let sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        audio_samples.push(sample); // left
                        audio_samples.push(sample); // right
                    }
                }
            }

            callbacks.upload_audio_frame(audio_samples.as_slice());
        }


        let framebuffer = self.emu.cpu_bus.read_full_framebuffer();
        self.framebuffer.video_frame = buffer_to_color_image(&framebuffer);

        let rendering_mode = self.rendering_mode.take().unwrap();
        let pixel_format = self.pixel_format.take().unwrap();
        
        callbacks.upload_video_frame(&rendering_mode, &pixel_format, &self.framebuffer);
        self.rendering_mode = Some(rendering_mode);
        self.pixel_format = Some(pixel_format);

        inputs_polled
    }

    fn reset(&mut self, env: &mut impl Reset) {
        self.emu.input_state.insert(InputCommand::HardReset, JustReleased);
    }

    fn unload_game(self, env: &mut impl UnloadGame) -> Self::Init {
        // self will be dropped here, so we can just return the default state
        Self::default()
    }
}

unsafe impl FrameBuffer for FrameBufferThing {
    type Pixel = ORGB1555;

    fn data(&self) -> &[u8] {
        &self.video_frame
    }

    fn width(&self) -> u16 {
        128
    }

    fn height(&self) -> u16 {
        128
    }
}

libretro_core!( crate::CoreEmulator );

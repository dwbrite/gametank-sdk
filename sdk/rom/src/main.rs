#![no_std]
#![no_main]
#![allow(unused)]
#![allow(static_mut_refs)]

use core::ptr;

use crate::{
    boot::{enable_irq_handler, wait},
    sdk::{
        audio::{pitch_table::MidiNote, wavetables::{VOLUME}}, scr::{Console, SystemControl}, via::Via, video_dma::blitter::BlitterGuard
    },
};

mod boot;
mod sdk;

static AUDIOFW: &[u8; 4096] = include_bytes!("../target/audiofw.bin");

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.bank126")]
fn draw_background(sc: &mut SystemControl, blitter: &mut BlitterGuard) {
    blitter.draw_square(sc, 1, 0, 127, 1, !0b100_00_000);
    blitter.wait_blit();
    blitter.draw_square(sc, 0, 1, 1, 127, !0b100_00_000);
    blitter.wait_blit();
    blitter.draw_square(sc, 0, 0, 1, 1, !0b100_00_000);
    blitter.draw_sprite(sc, 0, 0, 1, 1, 127, 127);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.bank125")]
fn fill_sprite_quad(console: &mut Console) {
    if let Some(mut sm) = console.dma.sprite_mem(&mut console.sc) {
        let mut color = 0;
        let mut x_counter: u8 = 0;
        let mut y_counter: u16 = 0;
        let mut xy_counter: u8 = 0;

        for (n, byte) in sm.bytes().iter_mut().enumerate() {
            if xy_counter == 16 {
                color += 32;
                xy_counter = 0;
            }
            if x_counter == 4 {
                color += 1;
                x_counter = 0;
            }
            if y_counter == 128 {
                color -= 32;
                xy_counter += 1;
                y_counter = 0;
            }
            *byte = color;
            x_counter += 1;
            y_counter += 1;
        }
    }
}

#[derive(Copy, Clone)]
struct Ball {
    x: i8,
    y: i8,
    vx: i8,
    vy: i8,
    size: u8,
    color: u8,
}

impl Ball {
    fn do_ball_things(&mut self) {
        let mut ball = self;
        let physics_size = ball.size;

        ball.x += ball.vx;
        ball.y += ball.vy;
        if ball.x >= (128 - physics_size) as i8 {
            ball.vx = -1;
        }
        if ball.x <= 33 {
            ball.vx = 1;
        }
        if ball.y >= (128 - physics_size) as i8 {
            ball.vy = -1;
        }
        if ball.y <= 1 {
            ball.vy = 1;
        }
    }

    fn draw(&self, sc: &mut SystemControl, blitter: &mut BlitterGuard) {
        let mut ball = self;
        blitter.draw_square(
            sc,
            ball.x as u8,
            ball.y as u8,
            ball.size as u8,
            ball.size,
            !ball.color,
        );
        blitter.wait_blit();
    }
}

// TODO: instead of doing console init - why not provide MAIN with a Console object, eh?
// Then we can even keep Console::init private and not need to make a singleton
#[unsafe(no_mangle)]
fn main(mut console: Console) {
    let via = unsafe { Via::new() };
    via.change_rom_bank(125);
    fill_sprite_quad(&mut console);

    console.audio.copy_from_slice(AUDIOFW);
    console.sc.set_audio(0xFF); // start playing audio at 14kHz

    let voices = sdk::audio::wavetables::voices();

    // we have to track the banks ourselves :^)
    via.change_rom_bank(126);

    let mut base_ball = Ball {
        x: 44,
        y: 19,
        size: 8,
        vx: 1,
        vy: 1,
        color: 0b010_11_100,
    };

    let mut balls = [base_ball; 6];
    for (n, ball) in balls.iter_mut().enumerate() {
        ball.size = (7 - n) as u8;
        ball.x -= (n << 0) as i8;
        ball.y -= (n << 0) as i8;
        ball.color += (n as u8) << 5;
    }

    let mut ctr = 0u16;
    let mut n = 0u8;
    let mut back_volume = 16usize;
    let mut front_volume = 16usize;

    loop {
        unsafe {
            wait();
        }
        if let Some(mut fb) = console.dma.framebuffers(&mut console.sc) {
            fb.flip(&mut console.sc);
        }

        let mut blitter = console.dma.blitter(&mut console.sc).unwrap();

        // We have a LOT of CPU cycles to use while drawing the full background, waiting for the blit to finish.
        draw_background(&mut console.sc, &mut blitter);

        // We can use that time to do all the physics and input handling _before_ drawing waiting for this blit to finish.
        for ball in &mut balls {
            ball.do_ball_things();
        }

        blitter.wait_blit();

        for ball in balls.iter().rev() {
            ball.draw(&mut console.sc, &mut blitter);
        }

        ctr += 1;
        if ctr == 60 {
            ctr = 0;
            n += 1;
        }

        match n {
            //  B C E G
            1 => {
                voices[0].set_tone(MidiNote::C4);
                voices[0].set_volume(VOLUME[back_volume]);
            }
            2 => {
                voices[1].set_tone(MidiNote::E4);
                voices[1].set_volume(VOLUME[back_volume]);
            }``
            3 => {
                voices[2].set_tone(MidiNote::G4);
                voices[2].set_volume(VOLUME[back_volume]);
            }
            4 => {
                voices[3].set_tone(MidiNote::B4);
                voices[3].set_volume(VOLUME[back_volume]);
            }
            5..6 => {
                voices[4].set_tone(MidiNote::D5);
                voices[4].set_volume(VOLUME[back_volume]);
            }
            6..10 => {
                voices[5].set_volume(VOLUME[front_volume]);

                if n == 8 {
                    match ctr {
                        0 => voices[5].set_tone(MidiNote::E5),
                        20 => voices[5].set_tone(MidiNote::B4),
                        40 => voices[5].set_tone(MidiNote::G4),
                        _ => {}
                    }
                }

                if back_volume <= 16 && back_volume > 0 && ctr % 16 == 0 {
                    back_volume -= 1;
                    
                    voices[4].set_volume(VOLUME[back_volume]);
                    voices[3].set_volume(VOLUME[back_volume]);
                    voices[2].set_volume(VOLUME[back_volume]);
                    voices[1].set_volume(VOLUME[back_volume]);
                    voices[0].set_volume(VOLUME[back_volume]);
                }
            }
            10..32 => {
                if front_volume <= 16 && front_volume > 0 && ctr % 4 == 0 {
                    front_volume -= 1;
                    voices[5].set_volume(VOLUME[front_volume]);
                }
            }
            _ => {}
        }
    }
}

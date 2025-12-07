#![no_std]
#![no_main]
#![allow(unused)]
#![allow(static_mut_refs)]

use crate::{
    ball::init_balls,
    boot::wait,
    sdk::{
        audio::FIRMWARE, scr::{Console, SystemControl}, via::Via, video_dma::blitter::BlitterGuard,
    },
};

use gametank_asset_macros::include_bmp;

mod audio_demo;
mod ball;
mod boot;
mod sdk;

#[unsafe(link_section = ".rodata.bank124")]
pub static GRADIENT_BACKGROUND: [u8; 128 * 128] = include_bmp!("assets/gradient.bmp");

fn load_background_sprite(console: &mut Console, via: &mut Via) {
    via.change_rom_bank(124);
    if let Some(mut sm) = console.dma.sprite_mem(&mut console.sc) {
        sm.bytes().copy_from_slice(&GRADIENT_BACKGROUND);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.bank126")]
fn draw_background(sc: &mut SystemControl, blitter: &mut BlitterGuard) {
    // Draw top border (127px wide, 1px tall, at x=1, y=0)
    blitter.draw_square(sc, 1, 0, 127, 1, !0b100_00_000);
    blitter.wait_blit();
    // Draw left border (1px wide, 127px tall, at x=0, y=1)
    blitter.draw_square(sc, 0, 1, 1, 127, !0b100_00_000);
    blitter.wait_blit();
    // Draw corner pixel at (0,0)
    blitter.draw_square(sc, 0, 0, 1, 1, !0b100_00_000);
    // Draw the gradient background sprite (127x127) at (1,1)
    blitter.draw_sprite(sc, 0, 0, 1, 1, 127, 127);
}

#[unsafe(no_mangle)]
fn main(mut console: Console) {
    let via = unsafe { Via::new() };

    load_background_sprite(&mut console, via);

    // load_background_sprite sets bank to 124, and our draw_background function is in bank 126
    via.change_rom_bank(126);

    console.audio.copy_from_slice(FIRMWARE);
    console.sc.set_audio(0xFF); // start playing audio at 14kHz

    let mut sequencer = audio_demo::init_demo();
    let mut balls = init_balls();

    loop {
        unsafe {
            wait();
        }

        if let Some(mut fb) = console.dma.framebuffers(&mut console.sc) {
            fb.flip(&mut console.sc);
        }

        let mut blitter = console.dma.blitter(&mut console.sc).unwrap();

        // the blitter runs parallel of the CPU, which means...
        draw_background(&mut console.sc, &mut blitter);

        // we can use that time to do other things, like
        // - physics
        for ball in &mut balls {
            ball.do_ball_things();
        }
        // - and audio sequencing!
        sequencer.tick();

        // then we wait for the blit to finish before drawing each ball
        blitter.wait_blit();
        for ball in balls.iter().rev() {
            ball.draw(&mut console.sc, &mut blitter);
        }
    }
}

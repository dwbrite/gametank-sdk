#![no_std]
#![no_main]
#![allow(unused)]
#![allow(static_mut_refs)]

use gametank::{
    audio::FIRMWARE, boot::wait, console::Console, via::Via, video_dma::blitter::BlitterGuard,
};

use crate::ball::init_balls;

use gametank_asset_macros::include_bmp;

mod audio_demo;
mod ball;

#[unsafe(link_section = ".rodata.bank124")]
pub static GRADIENT_BACKGROUND: [u8; 128 * 128] = include_bmp!("assets/gradient.bmp");

fn load_background_sprite(console: &mut Console) {
    console.via.change_rom_bank(124);
    if let Some(mut sm) = console.dma.sprite_mem(&mut console.video_flags) {
        sm.bytes().copy_from_slice(&GRADIENT_BACKGROUND);
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.bank126")]
fn draw_background(blitter: &mut BlitterGuard) {
    blitter.draw_sprite(0, 0, 0, 0, 127, 127);
}

#[unsafe(no_mangle)]
fn main(console: &mut Console) {
    load_background_sprite(console);

    // load_background_sprite sets bank to 124, and our draw_background function is in bank 126
    console.set_rom_bank(126);

    let mut sequencer = audio_demo::init_demo();
    let mut balls = init_balls();

    loop {
        unsafe {
            wait();
        }

        console.flip_framebuffers();

        // only unwrap when you know you have exclusive access to the blitter, dma, etc
        let mut blitter = console.blitter().unwrap();

        // the blitter runs parallel of the CPU, which means...
        draw_background(&mut blitter);

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
            ball.draw(&mut blitter);
        }

        // Apply letterbox to mask overscan areas before vsync
        blitter.draw_letterbox();
        blitter.wait_blit();
    }
}

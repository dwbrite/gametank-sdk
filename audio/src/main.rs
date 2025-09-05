#![no_std]
#![no_main]
#![allow(unused)]
#![allow(static_mut_refs)]

use core::ptr::{self};

use crate::{boot::{enable_irq_handler, return_from_interrupt, wait}, sine_table::{NES_CRUNCH, SINE_256}, vol::{Volume, VOLUME, VT}};

mod boot;
mod sine_table;
mod vol;

#[repr(C, packed)]
struct VoiceOptimized {
    phase_lo: u8,
    phase_hi: u8,
    freq_lo:  u8,
    freq_hi:  u8,
    wavetable: *const [u8; 256],
    volume: Volume,
}

impl VoiceOptimized {
    #[inline(always)]
    #[unsafe(no_mangle)]
    fn next_sample(&mut self) -> u8 {
        unsafe {
            add16(&mut self.phase_lo, &mut self.phase_hi, self.freq_lo, self.freq_hi);
            let idx: u8 = self.phase_hi;
            let s = (*self.wavetable).get_unchecked(idx as usize);     // (zp),Y on 65C02 if wavetable is in ZP
            self.volume.volume(*s)
        }
    }
}

#[inline(always)]
#[unsafe(no_mangle)]
fn add16(lo: &mut u8, hi: &mut u8, add_lo: u8, add_hi: u8) {
    let (nlo, c) = lo.overflowing_add(add_lo);
    *lo = nlo;
    *hi = hi.wrapping_add(add_hi).wrapping_add(c as u8);
}



#[derive(Clone, Copy)]
#[repr(C, packed)]
struct Voice {
    phase: u16,
    frequency: u16,
    wavetable: *const [u8; 256],
    volume: Volume,
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".data.voices")]
static mut VOICES: [Voice; 8] = [
Voice {
    phase: 0,
    frequency: 0,
    wavetable: &WAVETABLES[0],
    volume: VOLUME[0],
}; 8];


// impl Voice {
//     #[inline(always)]
//     fn next_sample(&mut self) -> u8 {
//         unsafe {
//             self.phase += self.frequency;
//             let sample = *(*self.wavetable).get_unchecked((self.phase >> 8) as usize);
//             self.volume.volume(sample)
//         }
//     }
// }

#[unsafe(no_mangle)]
#[unsafe(link_section = ".const.wavetables")]
static WAVETABLES: [[u8; 256]; 8] = [
    SINE_256,
    NES_CRUNCH,
    SINE_256,
    SINE_256,
    SINE_256,
    SINE_256,
    SINE_256,
    SINE_256,
];


#[unsafe(no_mangle)]
extern "C" fn audio_irq() {
    unsafe {
        let sample = &mut *(0x8040 as *mut u8);

        let mut sum = 0;

        for sin in &mut VOICES.iter_mut() {
            let tmp: &mut VoiceOptimized = core::mem::transmute(sin);
            sum += (tmp.next_sample() as u16);
        }

        let mixed = (sum >> 3) as u8;
        *sample = mixed;  // re-bias for DAC

        return_from_interrupt();
    }
}
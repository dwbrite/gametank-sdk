#![allow(non_camel_case_types)]

pub const FS: u32 = 13_983 / 2;           // samples/sec
const PHASE_MOD: u32 = 1 << 16;       // 16.16 phase accumulator
const SEMITONE_RATIO_Q16: u32 = 69_433; // ≈ 2^(1/12) * 65536
const MIDI0_FREQ_Q16: u32 = 535_400;  // 8.1757989156 Hz * 65536

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiNote {
    CNeg1, CsNeg1, DNeg1, DsNeg1, ENeg1, FNeg1, FsNeg1, GNeg1, GsNeg1, ANeg1, AsNeg1, BNeg1, // 0..11
    C0, Cs0, D0, Ds0, E0, F0, Fs0, G0, Gs0, A0, As0, B0,                                     // 12..23
    C1, Cs1, D1, Ds1, E1, F1, Fs1, G1, Gs1, A1, As1, B1,                                     // 24..35
    C2, Cs2, D2, Ds2, E2, F2, Fs2, G2, Gs2, A2, As2, B2,                                     // 36..47
    C3, Cs3, D3, Ds3, E3, F3, Fs3, G3, Gs3, A3, As3, B3,                                     // 48..59
    C4, Cs4, D4, Ds4, E4, F4, Fs4, G4, Gs4, A4, As4, B4,                                     // 60..71
    C5, Cs5, D5, Ds5, E5, F5, Fs5, G5, Gs5, A5, As5, B5,                                     // 72..83
    C6, Cs6, D6, Ds6, E6, F6, Fs6, G6, Gs6, A6, As6, B6,                                     // 84..95
    C7, Cs7, D7, Ds7, E7, F7, Fs7, G7, Gs7, A7, As7, B7,                                     // 96..107
    C8, Cs8, D8, Ds8, E8, F8, Fs8, G8, Gs8, A8, As8, B8,                                     // 108..119
    C9, Cs9, D9, Ds9, E9, F9, Fs9, G9,                                                       // 120..127
}

#[inline(always)]
pub const fn midi_inc(n: MidiNote) -> u16 {
    MIDI_INCREMENTS[n as u8 as usize]
}

#[inline(always)]
pub const fn hz_to_inc_q16(hz_q16: u32) -> u16 {
    // inc = round(hz * 65536 / FS) == round(hz_q16 / FS)
    ((hz_q16 as u64 + (FS as u64 / 2)) / (FS as u64)) as u16
}

const fn mul_q16(a: u32, b: u32) -> u32 {
    ((a as u64 * b as u64) >> 16) as u32
}

const fn build_table() -> [u16; 128] {
    let mut table = [0u16; 128];
    let mut freq_q16 = MIDI0_FREQ_Q16; // MIDI 0
    let mut i = 0;
    while i < 128 {
        table[i] = hz_to_inc_q16(freq_q16);
        freq_q16 = mul_q16(freq_q16, SEMITONE_RATIO_Q16);
        i += 1;
    }
    table
}

// inc table for FS=13_983 Hz, 16.16 phase (top 8 bits index 256-sample wavetable)
pub const MIDI_INCREMENTS: [u16; 128] = build_table();

// convenience: expected output Hz for a given increment (integer math, rounded)
#[inline(always)]
pub const fn inc_to_hz(inc: u16) -> u32 {
    // f = FS * inc / 65536
    ((FS as u64 * inc as u64 + (PHASE_MOD as u64 / 2)) / PHASE_MOD as u64) as u32
}

// sanity: inc=256 -> ~FS/256 ≈ ~54.6 Hz
pub const INC_256_HZ: u32 = inc_to_hz(256);

// pub const IDK: u16 = midi_inc(MidiNote::C5);

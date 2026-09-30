//! Hardware sample-rate register lookup.

/// Audio coprocessor clock frequency (NTSC colorburst-derived).
pub const CPU_FREQ: f64 = 3_579_545.0;

/// $D7 = ~20338Hz
pub const SAMPLE_RATE_REG: u8 = 0xD7;

/// Resolve a hardware `audio_freq` register value to its real
/// output sample rate in Hz.
pub const fn sample_rate_reg_to_hz(reg: u8) -> f64 {
    let low7 = (reg & 0x7F) as u32;
    let divisor = 2 * low7 + 1 + (low7 & 1);
    CPU_FREQ / divisor as f64
}

const FS: u32 = sample_rate_reg_to_hz(SAMPLE_RATE_REG) as u32;

#[inline(always)]
pub const fn hz_to_inc_q16(hz_q16: u32) -> u16 {
    ((hz_q16 as u64 + (FS as u64 / 2)) / (FS as u64)) as u16
}

const FS: u32 = 13_983; // samples/sec (14kHz interrupt rate)

#[inline(always)]
pub const fn hz_to_inc_q16(hz_q16: u32) -> u16 {
    // inc = round(hz * 65536 / FS) == round(hz_q16 / FS)
    ((hz_q16 as u64 + (FS as u64 / 2)) / (FS as u64)) as u16
}

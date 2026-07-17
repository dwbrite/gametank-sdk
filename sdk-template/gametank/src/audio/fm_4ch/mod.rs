//! # 4-Channel FM Synthesizer
//!
//! This firmware provides 4 independent FM channels, each driven by
//! 4 operators in series. It is a direct Rust port of the C SDK's
//! `audio_coprocessor.c` / `music.c` / `instruments.c` interface.
//!
//! ## Memory layout (ACP zero-page, CPU-side $3000 base)
//!
//! | Address  | Size | Purpose                                        |
//! |----------|------|------------------------------------------------|
//! | $3004    |  4 B | FeedbackAmount - one byte per channel          |
//! | $3010    | 16 B | FreqsH - one byte per operator (4×4)           |
//! | $3020    | 16 B | FreqsL                                         |
//! | $3030    | 16 B | BufferedAmplitudes - one byte per operator     |
//! | $3070    | 128B | Inputs - param write buffer `[addr,val,...,0]` |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gametank::audio::{FIRMWARE, channels, MidiNote};
//! use gametank::audio::fm_4ch::instruments::PIANO;
//!
//! console.sc.set_audio(0);
//! console.audio.copy_from_slice(FIRMWARE);
//! console.sc.set_audio(0xFF);
//!
//! let ch = channels();
//! ch[0].load_instrument(&PIANO);
//! ch[0].set_note(MidiNote::C4);
//! ch[0].note_on();
//!
//! // Each game frame:
//! ch[0].tick();   // decays operator amplitudes
//! flush_params(); // sends buffered writes to the ACP
//! ```
//!
//! ## Instruments
//!
//! See [`instruments`] for the predefined set (piano, guitar, etc.) that
//! mirror `instruments.c` in the C SDK.

use crate::audio::pitch_table::MidiNote;

pub mod instruments;
pub use instruments::Instrument;

/// ACP RAM base address as seen from the main CPU
const ARAM_BASE: usize = 0x3000;

/// `FeedbackAmount` - one byte per channel (channels 0-3)
/// Value encoding: `(feedback_amount << 3) + 128` (taken from C SDK)
const FEEDBACK_AMT: usize = 0x04;

/// `FreqsH` - high byte of pitch for each operator.
/// Layout: ops 0-3 = channel 0, ops 4-7 = channel 1, etc.
const PITCH_MSB: usize = 0x10;

/// `FreqsL` - low byte of pitch for each operator
const PITCH_LSB: usize = 0x20;

/// `BufferedAmplitudes` - current amplitude for each operator (0-255 internal scale)
/// Encoded as `(amplitude >> 1) + 128` when written to the ACP
const AMPLITUDE: usize = 0x30;

/// `Inputs` - NMI-driven parameter write buffer
/// Format: pairs of `[register_addr, value]`, terminated by `0x00`
const INPUTS: usize = 0x70;

pub const NUM_CHANNELS: usize = 4;
pub const OPS_PER_CHANNEL: usize = 4;
pub const NUM_OPS: usize = NUM_CHANNELS * OPS_PER_CHANNEL;

// ---------------------------------------------------------------------------
// Pitch table
// ---------------------------------------------------------------------------
//
// The C SDK's `pitch_table` is 216 bytes = 108 pairs of [MSB, LSB].
// It covers MIDI note offsets 0-107 (about C1-B9).
// set_note() uses: pitch_table[(op_transpose + note) * 2 .. +2]
//
// We embed the same table inline, matching `audio_coprocessor.c` exactly.

const PITCH_TABLE: [u8; 216] = [
    0x00, 0x4D, 0x00, 0x51, 0x00, 0x56, 0x00, 0x5B, 0x00, 0x61, 0x00, 0x66, 0x00, 0x6C, 0x00, 0x73,
    0x00, 0x7A, 0x00, 0x81, 0x00, 0x89, 0x00, 0x91, 0x00, 0x99, 0x00, 0xA2, 0x00, 0xAC, 0x00, 0xB6,
    0x00, 0xC1, 0x00, 0xCD, 0x00, 0xD9, 0x00, 0xE6, 0x00, 0xF3, 0x01, 0x02, 0x01, 0x11, 0x01, 0x21,
    0x01, 0x33, 0x01, 0x45, 0x01, 0x58, 0x01, 0x6D, 0x01, 0x82, 0x01, 0x99, 0x01, 0xB2, 0x01, 0xCB,
    0x01, 0xE7, 0x02, 0x04, 0x02, 0x22, 0x02, 0x43, 0x02, 0x65, 0x02, 0x8A, 0x02, 0xB0, 0x02, 0xD9,
    0x03, 0x04, 0x03, 0x32, 0x03, 0x63, 0x03, 0x97, 0x03, 0xCD, 0x04, 0x07, 0x04, 0x44, 0x04, 0x85,
    0x04, 0xCA, 0x05, 0x13, 0x05, 0x60, 0x05, 0xB2, 0x06, 0x09, 0x06, 0x65, 0x06, 0xC6, 0x07, 0x2D,
    0x07, 0x9A, 0x08, 0x0E, 0x08, 0x89, 0x09, 0x0B, 0x09, 0x94, 0x0A, 0x26, 0x0A, 0xC1, 0x0B, 0x64,
    0x0C, 0x12, 0x0C, 0xCA, 0x0D, 0x8C, 0x0E, 0x5B, 0x0F, 0x35, 0x10, 0x1D, 0x11, 0x12, 0x12, 0x16,
    0x13, 0x29, 0x14, 0x4D, 0x15, 0x82, 0x16, 0xC9, 0x18, 0x24, 0x19, 0x93, 0x1B, 0x19, 0x1C, 0xB5,
    0x1E, 0x6A, 0x20, 0x39, 0x22, 0x24, 0x24, 0x2B, 0x26, 0x52, 0x28, 0x99, 0x2B, 0x03, 0x2D, 0x92,
    0x30, 0x48, 0x33, 0x27, 0x36, 0x31, 0x39, 0x6A, 0x3C, 0xD4, 0x40, 0x72, 0x44, 0x47, 0x48, 0x57,
    0x4C, 0xA4, 0x51, 0x32, 0x56, 0x06, 0x5B, 0x24, 0x60, 0x8F, 0x66, 0x4D, 0x6C, 0x62, 0x72, 0xD4,
    0x79, 0xA8, 0x80, 0xE4, 0x88, 0x8E, 0x90, 0xAD,
];

/// Write a byte into ACP RAM directly (no NMI buffering).
///
/// Equivalent to `set_audio_param(param, value)` / `aram[param] = value`
/// in the C SDK. Use [`flush_params`] for buffered NMI writes.
#[inline]
fn aram_write(param: usize, value: u8) {
    unsafe {
        core::ptr::write_volatile((ARAM_BASE + param) as *mut u8, value);
    }
}

/// Read a byte from ACP RAM.
#[allow(dead_code)]
#[inline]
fn aram_read(param: usize) -> u8 {
    unsafe { core::ptr::read_volatile((ARAM_BASE + param) as *const u8) }
}

// ---------------------------------------------------------------------------
// Param buffer (Inputs / NMI write path)
// ---------------------------------------------------------------------------
//
// The C SDK uses a static `audio_params_index` counter and `push_audio_param`
// / `flush_audio_params`. We mirror this with a module-level static.

static mut PARAM_IDX: u8 = 0;

/// Buffer a `(register, value)` pair for delivery to the ACP on the next NMI.
#[inline]
pub fn push_param(param: u8, value: u8) {
    unsafe {
        let idx = PARAM_IDX as usize;
        aram_write(INPUTS + idx,     param);
        aram_write(INPUTS + idx + 1, value);
        PARAM_IDX += 2;
    }
}

/// Hardware `audio_nmi` register (CPU-side, matches `AudioManager::audio_nmi`
/// at $2001). Writing `1` here fires the ACP's NMI, which drains the
/// `Inputs` buffer into its zero page. This is a different address than
/// anything inside ACP RAM ($3000-$3FFF).
const AUDIO_NMI_REG: *mut u8 = 0x2001 as *mut u8;

/// Terminate the param buffer and trigger an ACP NMI to process it.
#[inline]
pub fn flush_params() {
    unsafe {
        let idx = PARAM_IDX as usize;
        aram_write(INPUTS + idx, 0); // terminator
        core::ptr::write_volatile(AUDIO_NMI_REG, 1); // trigger NMI
        PARAM_IDX = 0;
    }
}

// ---------------------------------------------------------------------------
// Per-operator envelope state (mirrors music.c globals)
// ---------------------------------------------------------------------------

static mut AUDIO_AMPLITUDES: [u8; NUM_OPS]  = [0; NUM_OPS];
static mut ENV_INITIAL:       [u8; NUM_OPS]  = [0; NUM_OPS];
static mut ENV_DECAY:         [u8; NUM_OPS]  = [0; NUM_OPS];
static mut ENV_SUSTAIN:       [u8; NUM_OPS]  = [0; NUM_OPS];
static mut OP_TRANSPOSE:      [u8; NUM_OPS]  = [0; NUM_OPS];
static mut CH_NOTE_OFFSET:    [i8; NUM_CHANNELS] = [0; NUM_CHANNELS];

/// Per-channel "note held" flag, mirroring the C SDK's `note_held_mask`.
///
/// Envelope decay in [`Channel::tick`] only runs for channels with an active
/// held note - without this gate, a silenced channel's amplitude (0) reads
/// as "below sustain" on the very next tick and gets driven back up by the
/// decay formula, causing the note to revive and sustain indefinitely
/// instead of staying silent.
static mut NOTE_HELD: [bool; NUM_CHANNELS] = [false; NUM_CHANNELS];

// ---------------------------------------------------------------------------
// Channel API
// ---------------------------------------------------------------------------

/// One FM channel (4 operators).
///
/// Addresses are derived from the channel index at runtime; this is a
/// zero-sized token type - no data is stored in Rust memory. All state
/// lives directly in ACP RAM and the module-level static arrays.
pub struct Channel {
    idx: usize,
}

impl Channel {
    /// Load an instrument definition onto this channel.
    ///
    /// Mirrors `load_instrument(channel, instr)` in `music.c`. Writes:
    /// - `FeedbackAmount[ch]` -> ACP RAM directly
    /// - per-operator envelope arrays (env_initial, env_decay, env_sustain, op_transpose)
    /// - channel note offset (transpose)
    pub fn load_instrument(&mut self, instr: &Instrument) {
        let ch = self.idx;
        unsafe {
            CH_NOTE_OFFSET[ch] = instr.transpose;

            // Feedback: C SDK formula is `(feedback << 3) + 128`
            aram_write(FEEDBACK_AMT + ch, (instr.feedback << 3).wrapping_add(128));

            let op_base = ch * OPS_PER_CHANNEL;
            for i in 0..OPS_PER_CHANNEL {
                ENV_INITIAL [op_base + i] = instr.env_initial [i];
                ENV_DECAY   [op_base + i] = instr.env_decay   [i];
                ENV_SUSTAIN [op_base + i] = instr.env_sustain [i];
                OP_TRANSPOSE[op_base + i] = instr.op_transpose[i];
            }
        }
    }

    /// Trigger a note-on event, setting all operators to their initial amplitude.
    ///
    /// Mirrors the note-on path inside `tick_music()` in `music.c`.
    pub fn note_on(&mut self) {
        let op_base = self.idx * OPS_PER_CHANNEL;
        unsafe {
            NOTE_HELD[self.idx] = true;
            for i in 0..OPS_PER_CHANNEL {
                AUDIO_AMPLITUDES[op_base + i] = ENV_INITIAL[op_base + i];
                push_param(
                    (AMPLITUDE + op_base + i) as u8,
                    (AUDIO_AMPLITUDES[op_base + i] >> 1).wrapping_add(128),
                );
            }
        }
    }

    /// Trigger a note-off: silence the carrier operator (op 3) and stop
    /// further envelope decay on this channel.
    ///
    /// Mirrors `audio_amplitudes[op+3] = 0` plus clearing `note_held_mask`
    /// for this channel in `tick_music()`.
    pub fn note_off(&mut self) {
        let op = self.idx * OPS_PER_CHANNEL + 3;
        unsafe {
            NOTE_HELD[self.idx] = false;
            AUDIO_AMPLITUDES[op] = 0;
            push_param((AMPLITUDE + op) as u8, 128);
        }
    }

    /// Set the pitch for this channel from a MIDI note.
    ///
    /// Applies `CH_NOTE_OFFSET` (instrument transpose) and the per-operator
    /// `OP_TRANSPOSE` offsets, then writes PITCH_MSB and PITCH_LSB for all
    /// 4 operators via the param buffer.
    ///
    /// Mirrors `set_note(ch, n)` in `music.c`.
    pub fn set_note(&mut self, note: MidiNote) {
        let base_note = (note as u8 as i16 + unsafe { CH_NOTE_OFFSET[self.idx] } as i16)
            .clamp(0, 107) as u8;
        let op_base = self.idx * OPS_PER_CHANNEL;

        for i in 0..OPS_PER_CHANNEL {
            let op = op_base + i;
            let transposed = (base_note as i16
                + unsafe { OP_TRANSPOSE[op] } as i16)
                .clamp(0, 107) as usize;
            let idx = transposed * 2;
            push_param((PITCH_MSB + op) as u8, PITCH_TABLE[idx]);
            push_param((PITCH_LSB + op) as u8, PITCH_TABLE[idx + 1]);
        }
    }

    /// Advance the envelope for this channel by one frame (decay toward sustain).
    ///
    /// Mirrors the per-channel envelope decay loop in `tick_music()`, which
    /// only runs for channels with an active held note (`note_held_mask`).
    /// Skipping this gate would let silenced channels' amplitudes get pulled
    /// back up toward their (non-zero) sustain level every frame.
    pub fn tick(&mut self) {
        if unsafe { !NOTE_HELD[self.idx] } {
            return;
        }
        let op_base = self.idx * OPS_PER_CHANNEL;
        unsafe {
            for i in 0..OPS_PER_CHANNEL {
                let op = op_base + i;
                let amp  = &mut AUDIO_AMPLITUDES[op];
                let sus  = ENV_SUSTAIN[op];
                let dec  = ENV_DECAY[op];

                // C SDK: if ((sustain - amp) ^ decay) & 0x80 → decay still needed
                if ((sus.wrapping_sub(*amp)) ^ dec) & 0x80 != 0 {
                    *amp = amp.wrapping_sub(dec);
                } else {
                    *amp = sus;
                }
                push_param((AMPLITUDE + op) as u8, (*amp >> 1).wrapping_add(128));
            }
        }
    }

    /// Silence all operators on this channel immediately.
    pub fn silence(&mut self) {
        let op_base = self.idx * OPS_PER_CHANNEL;
        unsafe {
            NOTE_HELD[self.idx] = false;
            for i in 0..OPS_PER_CHANNEL {
                let op = op_base + i;
                AUDIO_AMPLITUDES[op] = 0;
                push_param((AMPLITUDE + op) as u8, 128);
            }
        }
    }
}

/// Get mutable references to all 4 FM channels.
///
/// # Safety
/// Creates channel tokens backed by memory-mapped ACP registers.
/// Caller must ensure no aliasing between channel indices.
pub fn channels() -> [Channel; NUM_CHANNELS] {
    [
        Channel { idx: 0 },
        Channel { idx: 1 },
        Channel { idx: 2 },
        Channel { idx: 3 },
    ]
}

/// Silence all channels and reset all operator amplitudes.
///
/// Mirrors `silence_all_channels()` in `music.c`.
pub fn silence_all() {
    unsafe {
        NOTE_HELD = [false; NUM_CHANNELS];
        for op in 0..NUM_OPS {
            AUDIO_AMPLITUDES[op] = 0;
            push_param((AMPLITUDE + op) as u8, 128);
        }
    }
}

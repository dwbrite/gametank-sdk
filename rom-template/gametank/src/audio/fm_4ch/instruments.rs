//! Predefined FM instrument definitions
//!
//! Each [`Instrument`] encodes the 4-operator envelope and pitch configuration
//! for one of the C SDK's named instruments from `instruments.c`.
//!
//! Field layout mirrors the C `Instrument` struct:
//! ```c
//! typedef struct {
//!     unsigned char env_initial[OPS_PER_CHANNEL];
//!     unsigned char env_decay[OPS_PER_CHANNEL];
//!     unsigned char env_sustain[OPS_PER_CHANNEL];
//!     unsigned char op_transpose[OPS_PER_CHANNEL];
//!     unsigned char feedback;
//!     signed char   transpose;
//! } Instrument;
//! ```

/// A 4-operator FM instrument definition
///
/// All amplitude fields (`env_initial`, `env_decay`, `env_sustain`) use the
/// internal 0-255 ACP scale. The hardware receives `(amplitude >> 1) + 128`,
/// which maps to the signed ±63 range expected by the sine-based FM engine.
#[derive(Clone, Copy)]
pub struct Instrument {
    /// Peak amplitude at note onset for each operator (0-255).
    pub env_initial:  [u8; 4],
    /// Per-frame amplitude decay for each operator.
    pub env_decay:    [u8; 4],
    /// Amplitude floor after decay for each operator.
    pub env_sustain:  [u8; 4],
    /// Semitone offset added to the channel note for each operator.
    /// Controls FM detuning / harmonic ratios between operators.
    pub op_transpose: [u8; 4],
    /// Self-feedback amount for operator 0.
    /// Encoded as `(feedback << 3) + 128` when written to ACP.
    pub feedback: u8,
    /// Semitone transpose applied to the whole channel.
    pub transpose: i8,
}

/// Piano: bright attack, decays to a medium sustained carrier
pub const PIANO: Instrument = Instrument {
    env_initial:  [0x30, 0x40, 0x40, 0x5f],
    env_decay:    [0x04, 0x02, 0x10, 0x02],
    env_sustain:  [0x04, 0x02, 0x10, 0x30],
    op_transpose: [0, 0, 0, 0],
    feedback:     0,
    transpose:    0,
};

/// Guitar: plucked string, fast decay to silence on the carrier
pub const GUITAR: Instrument = Instrument {
    env_initial:  [0x6f, 0x40, 0x68, 0x5f],
    env_decay:    [0x00, 0xFF, 0x02, 0x08],
    env_sustain:  [0x00, 0x00, 0x40, 0x08],
    op_transpose: [12, 36, 0, 24],
    feedback:     8,
    transpose:    -12,
};

/// Distorted guitar: similar to guitar, slower carrier decay, lower sustain floor
pub const DIST_GUITAR: Instrument = Instrument {
    env_initial:  [0x60, 0x40, 0x88, 0x4f],
    env_decay:    [0x00, 0xFF, 0x02, 0x01],
    env_sustain:  [0x00, 0x00, 0x40, 0x30],
    op_transpose: [12, 36, 0, 24],
    feedback:     8,
    transpose:    -12,
};

/// Slap bass: punchy transient, decays to a low sustained level, two octaves down
pub const SLAP_BASS: Instrument = Instrument {
    env_initial:  [0x58, 0x88, 0x58, 0x5f],
    env_decay:    [0x18, 0x08, 0x04, 0x02],
    env_sustain:  [0x18, 0x08, 0x04, 0x02],
    op_transpose: [28, 12, 0, 12],
    feedback:     0,
    transpose:    -24,
};

/// Snare drum: noisy burst via high-ratio op detune, decays quickly
pub const SNARE: Instrument = Instrument {
    env_initial:  [0x88, 0x8f, 0x8f, 0x38],
    env_decay:    [0x18, 0x02, 0x04, 0x04],
    env_sustain:  [0x18, 0x08, 0x08, 0x04],
    op_transpose: [36, 0, 0, 0],
    feedback:     8,
    transpose:    -8,
};

/// Sitar: bright pluck, very slow fade, complex operator tuning
pub const SITAR: Instrument = Instrument {
    env_initial:  [0x60, 0x40, 0x01, 0x10],
    env_decay:    [0x00, 0xFF, 0xF8, 0xFF],
    env_sustain:  [0x00, 0x60, 0x60, 0x30],
    op_transpose: [12, 36, 12, 24],
    feedback:     4,
    transpose:    -24,
};

/// Horn / pad: slow attack operators, steady sustain, one octave down
pub const HORN: Instrument = Instrument {
    env_initial:  [0x00, 0x00, 0x01, 0x10],
    env_decay:    [0x00, 0x00, 0xFC, 0xFC],
    env_sustain:  [0x00, 0x00, 0x30, 0x50],
    op_transpose: [12, 36, 12, 24],
    feedback:     0,
    transpose:    -12,
};

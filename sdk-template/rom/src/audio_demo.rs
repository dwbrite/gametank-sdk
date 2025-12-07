//! Audio demo module - example chord progressions and sequencing
//!
//! This module demonstrates how to use the wavetable-8v audio firmware.

use crate::sdk::audio::{voices, MidiNote, WAVETABLE};

/// Volume levels - using indices like the original (0 = silent, 16 = max)
/// Each step is roughly 2 raw volume units
const fn vol(level: u8) -> u8 {
    if level > 16 { 32 } else { level * 2 }
}

/// Sequencer state for the demo
pub struct DemoSequencer {
    /// Frame counter (resets every 60 frames = 1 second at 60fps)
    frame: u16,
    /// Current step in the sequence
    step: u8,
    /// Background chord volume level (0-16 scale like original)
    bg_level: u8,
    /// Melody voice volume level (0-16 scale like original)
    melody_level: u8,
}

impl DemoSequencer {
    pub const fn new() -> Self {
        Self {
            frame: 0,
            step: 0,
            bg_level: 16,
            melody_level: 16,
        }
    }

    /// Call once per frame (60fps). Advances the sequence.
    pub fn tick(&mut self) {
        let v = voices();

        // Process current step BEFORE incrementing (matches original timing)
        match self.step {
            // Build up Cmaj7 chord, one note per second
            1 => {
                if self.frame == 0 {
                    v[0].set_note(MidiNote::C4);
                    v[0].set_volume(vol(self.bg_level));
                }
            }
            2 => {
                if self.frame == 0 {
                    v[1].set_note(MidiNote::E4);
                    v[1].set_volume(vol(self.bg_level));
                }
            }
            3 => {
                if self.frame == 0 {
                    v[2].set_note(MidiNote::G4);
                    v[2].set_volume(vol(self.bg_level));
                }
            }
            4 => {
                if self.frame == 0 {
                    v[3].set_note(MidiNote::B4);
                    v[3].set_volume(vol(self.bg_level));
                }
            }
            // Step 5: Add D5
            5 => {
                if self.frame == 0 {
                    v[4].set_note(MidiNote::D5);
                    v[4].set_volume(vol(self.bg_level));
                }
            }

            // Steps 6-9: Arpeggio melody on voice 5, fade background
            6..=9 => {
                // Start melody voice at step 6
                if self.step == 6 && self.frame == 0 {
                    v[5].set_volume(vol(self.melody_level));
                }

                // Play arpeggio pattern during step 8
                if self.step == 8 {
                    match self.frame {
                        0 => v[5].set_note(MidiNote::E5),
                        20 => v[5].set_note(MidiNote::B4),
                        40 => v[5].set_note(MidiNote::G4),
                        _ => {}
                    }
                }

                // Fade out background chord (every 16 frames, decrement level)
                if self.bg_level > 0 && self.frame % 16 == 0 {
                    self.bg_level -= 1;
                    let vol_val = vol(self.bg_level);
                    v[0].set_volume(vol_val);
                    v[1].set_volume(vol_val);
                    v[2].set_volume(vol_val);
                    v[3].set_volume(vol_val);
                    v[4].set_volume(vol_val);
                }
            }

            // Fade out melody
            10..=31 => {
                if self.melody_level > 0 && self.frame % 4 == 0 {
                    self.melody_level -= 1;
                    v[5].set_volume(vol(self.melody_level));
                }
            }

            // Sequence complete
            _ => {}
        }

        // Increment counters AFTER processing (matches original)
        self.frame += 1;
        if self.frame >= 60 {
            self.frame = 0;
            self.step += 1;
        }
    }
}

/// Initialize voices for the demo (set wavetables, mute all)
pub fn init_demo() -> DemoSequencer {
    let v = voices();

    // Set all voices to use the first wavetable (sine) and mute
    for voice in v.iter_mut() {
        voice.set_wavetable(WAVETABLE[0]);
        voice.set_volume(0);
    }

    DemoSequencer::new()
}

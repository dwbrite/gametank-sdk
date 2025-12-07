//! # GameTank SDK
//!
//! The Rust SDK for developing games on the [GameTank](https://gametank.zone/) console.
//!
//! ## Quick Start
//!
//! Every GameTank program starts with a `main` function that receives a [`Console`](scr::Console):
//!
//! ```ignore
//! use rom::sdk::{scr::Console, via::Via};
//! use rom::boot::wait;
//!
//! #[unsafe(no_mangle)]
//! fn main(mut console: Console) {
//!     let via = unsafe { Via::new() };
//!     
//!     loop {
//!         unsafe { wait(); }  // Wait for vblank (60 Hz)
//!         
//!         // Flip buffers so we draw to the hidden one
//!         if let Some(fb) = console.dma.framebuffers(&mut console.sc) {
//!             fb.flip(&mut console.sc);
//!         }
//!         
//!         // Draw a red rectangle
//!         let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//!         blitter.draw_square(&mut console.sc, 10, 10, 32, 32, !0b010_11_100);
//!         blitter.wait_blit();
//!     }
//! }
//! ```
//!
//! ## The Game Loop
//!
//! A typical frame looks like this:
//!
//! ```ignore
//! loop {
//!     // 1. Wait for vblank (TV finished drawing previous frame)
//!     unsafe { wait(); }
//!     
//!     // 2. Flip framebuffers (swap which buffer is displayed vs drawn to)
//!     console.dma.framebuffers(&mut console.sc).unwrap().flip(&mut console.sc);
//!     
//!     // 3. Start drawing background (blitter runs in parallel!)
//!     let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//!     blitter.draw_sprite(&mut console.sc, 0, 0, 0, 0, 128, 128);
//!     
//!     // 4. Do CPU work WHILE blitter draws (this is free parallelism!)
//!     update_game_logic();
//!     read_input();
//!     
//!     // 5. Wait for background to finish, then draw sprites on top
//!     blitter.wait_blit();
//!     for sprite in &sprites {
//!         blitter.draw_sprite(&mut console.sc, ...);
//!         blitter.wait_blit();
//!     }
//! }
//! ```
//!
//! ## Drawing
//!
//! All drawing goes through the [`BlitterGuard`](video_dma::blitter::BlitterGuard):
//!
//! ```ignore
//! let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//!
//! // Fill a rectangle with a solid color
//! blitter.draw_square(&mut console.sc, x, y, width, height, !color);
//!
//! // Copy a sprite from sprite RAM to the screen
//! blitter.draw_sprite(&mut console.sc, src_x, src_y, dst_x, dst_y, width, height);
//!
//! // IMPORTANT: Wait before starting another blit or accessing video memory
//! blitter.wait_blit();
//! ```
//!
//! ## Colors
//!
//! Colors are 8-bit HSL: `0bHHH_SS_LLL` (Hue, Saturation, Luminosity)
//!
//! ```ignore
//! // Common colors
//! const BLACK: u8  = 0b000_00_000;
//! const WHITE: u8  = 0b000_00_111;
//! const RED: u8    = 0b010_11_100;
//! const GREEN: u8  = 0b111_11_100;
//! const BLUE: u8   = 0b101_11_100;
//! const YELLOW: u8 = 0b000_11_100;
//!
//! // Hues: 0=Yellow, 1=Orange, 2=Red, 3=Magenta, 4=Violet, 5=Blue, 6=Cyan, 7=Green
//! // Saturation 0 = grayscale
//!
//! // IMPORTANT: Invert colors when drawing!
//! blitter.draw_square(&mut console.sc, x, y, w, h, !RED);
//! ```
//!
//! ## Loading Sprites
//!
//! Before you can draw sprites, load them into sprite RAM:
//!
//! ```ignore
//! // Your sprite data (typically from an asset macro)
//! static SPRITES: &[u8] = include_bytes!("sprites.bin");
//!
//! // Get sprite RAM access
//! let mut sprite_mem = console.dma.sprite_mem(&mut console.sc).unwrap();
//!
//! // Copy sprite data
//! sprite_mem.bytes()[..SPRITES.len()].copy_from_slice(SPRITES);
//! ```
//!
//! ## Audio
//!
//! The GameTank has a dedicated audio coprocessor. Initialize it with firmware:
//!
//! ```ignore
//! use rom::sdk::audio::{FIRMWARE, voices, MidiNote, WAVETABLE};
//!
//! // Load audio firmware (do this once at startup)
//! console.sc.set_audio(0);  // Disable while loading
//! console.audio.copy_from_slice(FIRMWARE);
//! console.sc.set_audio(0xFF);  // Enable at ~14kHz
//!
//! // Play notes using the wavetable synth
//! let v = voices();
//! v[0].set_note(MidiNote::C4);
//! v[0].set_volume(63);
//! v[0].set_wavetable(WAVETABLE[0]);
//! ```
//!
//! ## ROM Banking
//!
//! For large games, store assets in ROM banks and switch as needed:
//!
//! ```ignore
//! // Place data in a specific bank
//! #[unsafe(link_section = ".rodata.bank10")]
//! static LEVEL_DATA: [u8; 8192] = [...];
//!
//! // Switch to that bank before accessing
//! let via = unsafe { Via::new() };
//! via.change_rom_bank(10);
//! // Now LEVEL_DATA is accessible at its address
//! ```
//!
//! ## Hardware Overview
//!
//! | Feature | Spec |
//! |---------|------|
//! | CPU | W65C02S @ ~3.58 MHz |
//! | Display | 128×128 pixels, ~200 colors |
//! | Blitter | ~60,000 pixels/frame (3.6× screen) |
//! | RAM | 32KB (4 × 8KB banks) |
//! | Sprite RAM | 512KB (8 pages × 256×256) |
//! | ROM | 2MB (128 × 16KB banks) |
//! | Audio | 6502 coprocessor, 8-bit DAC, ~14kHz |

pub mod blitter;
pub mod scr;
pub mod via;
pub mod video_dma;
pub mod audio;

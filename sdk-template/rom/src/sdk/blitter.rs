//! # Low-Level Blitter Control Registers
//!
//! Direct access to the Blitter Control Registers (BCR) at `$4000-$4007`.
//!
//! **Note:** For most use cases, use [`video_dma::blitter::BlitterGuard`](crate::sdk::video_dma::blitter::BlitterGuard)
//! instead, which provides a safer and more ergonomic interface.
//!
//! ## Register Layout
//!
//! | Address | Name    | Description                                    |
//! |---------|---------|------------------------------------------------|
//! | `$4000` | VX      | Framebuffer X coordinate (destination)         |
//! | `$4001` | VY      | Framebuffer Y coordinate (destination)         |
//! | `$4002` | GX      | Sprite RAM X coordinate (source)               |
//! | `$4003` | GY      | Sprite RAM Y coordinate (source)               |
//! | `$4004` | WIDTH   | Width of rectangle (bit 7 = horizontal flip)   |
//! | `$4005` | HEIGHT  | Height of rectangle (bit 7 = vertical flip)    |
//! | `$4006` | START   | Write 1 to start blit, 0 to acknowledge IRQ    |
//! | `$4007` | COLOR   | Fill color (inverted, for color fill mode)     |
//!
//! ## Blitter Performance
//!
//! The blitter copies **1 pixel per CPU cycle** (~3.58 MHz). At 60 Hz:
//! - **~59,659 pixels/frame** theoretical maximum
//! - That's **~3.6× the framebuffer size** (128×128 = 16,384 pixels)
//!
//! Transparent pixels are still processed (skipped but counted), so large
//! transparent regions don't save time.
//!
//! ## Sprite Quadrants
//!
//! Sprite RAM is **512KB**: 8 pages of 256×256 pixels (64KB each). The page is selected
//! by bits 0-2 of the Banking Register. The blitter can access the full 256×256 page,
//! but the CPU can only access one **128×128 quadrant** at a time.
//!
//! The CPU-accessible quadrant is determined by the MSB of the blitter's GX/GY counters.
//! Use [`SpriteQuadrant`] to set which quadrant is accessible before loading sprites.

use volatile_register::WO;

use crate::{
    boot::{wait},
    sdk::{scr::SystemControl, video_dma::blitter::Blitter},
};

/// Blitter Control Register hardware layout at `$4000-$4007`.
///
/// Write-only registers that control blitter DMA operations.
/// The blitter copies rectangular regions from sprite RAM to the framebuffer,
/// or fills rectangles with a solid color.
#[repr(C, packed)]
pub struct Bcr {
    /// Framebuffer X destination (0-127).
    pub fb_x: WO<u8>,
    /// Framebuffer Y destination (0-127).
    pub fb_y: WO<u8>,
    /// Sprite RAM X source (0-255).
    pub vram_x: WO<u8>,
    /// Sprite RAM Y source (0-255+).
    pub vram_y: WO<u8>,
    /// Width of the blit operation.
    pub width: WO<u8>,
    /// Height of the blit operation.
    pub height: WO<u8>,
    /// Write 1 to start the blit, write 0 to acknowledge completion.
    pub start: WO<u8>,
    /// Fill color (inverted). Only used when `DMA_COLORFILL` is set.
    pub color: WO<u8>,
}

impl Bcr {
    /// Get a reference to the BCR at `$4000`.
    #[inline(always)]
    pub(in crate::sdk) unsafe fn new() -> &'static mut Bcr {
        unsafe { &mut *(0x4000 as *mut Bcr) }
    }
}

/// Blitter fill mode.
#[derive(PartialEq)]
pub enum BlitterFillMode {
    /// Copy pixels from sprite RAM to framebuffer.
    Sprite,
    /// Fill with a solid color.
    Color,
}

/// Sprite RAM quadrant selector.
///
/// Each sprite page is 256×256 pixels, but the CPU can only access one 128×128
/// quadrant at a time through `$4000-$7FFF`. The quadrant is determined by the
/// MSB of the blitter's GX/GY counters.
///
/// Use [`BlitterGuard::set_vram_quad`](crate::sdk::video_dma::blitter::BlitterGuard::set_vram_quad)
/// to select a quadrant before loading sprite data.
///
/// ```text
/// Sprite page quadrants (256×256 page):
/// ┌───────────┬───────────┐
/// │ Quadrant 1│ Quadrant 2│  Y = 0-127
/// │  (0,0)    │ (128,0)   │
/// ├───────────┼───────────┤
/// │ Quadrant 3│ Quadrant 4│  Y = 128-255
/// │  (0,128)  │ (128,128) │
/// └───────────┴───────────┘
///   X=0-127     X=128-255
/// ```
pub enum SpriteQuadrant {
    /// Top-left (X: 0-127, Y: 0-127)
    One,
    /// Top-right (X: 128-255, Y: 0-127)
    Two,
    /// Bottom-left (X: 0-127, Y: 128-255)
    Three,
    /// Bottom-right (X: 128-255, Y: 128-255)
    Four,
}

impl SpriteQuadrant {
    /// Get the X offset for this quadrant (0 or 128).
    #[inline(always)]
    pub fn value_gx(&self) -> u8 {
        match self {
            Self::One | Self::Three => 0,
            Self::Two | Self::Four => 128,
        }
    }

    /// Get the Y offset for this quadrant (0 or 128).
    #[inline(always)]
    pub fn value_gy(&self) -> u8 {
        match self {
            Self::One | Self::Two => 0,
            Self::Three | Self::Four => 128,
        }
    }
}

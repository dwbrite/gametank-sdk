//! # Video & Drawing
//!
//! This module provides access to the GameTank's video hardware through
//! [`DmaManager`], which you access via `console.dma`.
//!
//! ## Drawing with the Blitter
//!
//! The blitter is a hardware DMA engine that draws **in parallel** with the CPU.
//! This is the main way to draw graphics:
//!
//! ```ignore
//! let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//!
//! // Fill rectangles with solid colors
//! blitter.draw_square(&mut console.sc, x, y, width, height, !color);
//!
//! // Copy sprites from sprite RAM to screen
//! blitter.draw_sprite(&mut console.sc, src_x, src_y, dst_x, dst_y, w, h);
//!
//! // IMPORTANT: Wait before the next draw or before accessing video memory
//! blitter.wait_blit();
//! ```
//!
//! ### Parallel Execution (Free Performance!)
//!
//! The blitter runs independently of the CPU. Start a large blit, then do
//! CPU work while it draws:
//!
//! ```ignore
//! // Start drawing the background (128×128 = 16K pixels)
//! blitter.draw_sprite(&mut console.sc, 0, 0, 0, 0, 128, 128);
//!
//! // These run IN PARALLEL with the blit - essentially "free" CPU time!
//! update_physics();
//! process_input();
//! update_animations();
//!
//! // Now wait for the blit to finish before drawing more
//! blitter.wait_blit();
//! ```
//!
//! ## Double Buffering
//!
//! The GameTank has two framebuffers. While one is displayed, you draw to the other:
//!
//! ```ignore
//! loop {
//!     unsafe { wait(); }  // Wait for vblank
//!     
//!     // Flip: the buffer we drew to is now displayed,
//!     // and we'll draw to the previously-displayed one
//!     console.dma.framebuffers(&mut console.sc).unwrap().flip(&mut console.sc);
//!     
//!     // Now draw the next frame...
//! }
//! ```
//!
//! ## Loading Sprites
//!
//! Before you can draw sprites, load graphics into sprite RAM:
//!
//! ```ignore
//! let mut sm = console.dma.sprite_mem(&mut console.sc).unwrap();
//! sm.bytes()[..my_sprites.len()].copy_from_slice(my_sprites);
//! ```
//!
//! Sprite RAM has 8 pages of 256×256 pixels each (512KB total).
//! Select the page with [`BankFlags`](super::scr::BankFlags).

pub mod blitter;
pub mod framebuffers;
pub mod spritemem;

use crate::sdk::{
    scr::{SystemControl, VideoFlags},
    video_dma::{
        blitter::{Blitter, BlitterGuard},
        framebuffers::{Framebuffers, FramebuffersGuard},
        spritemem::{SpriteMem, SpriteMemGuard},
    },
};

// DMA_ENABLE == 0 -> CPU can see video memory
//   DMA_CPU_TO_VRAM == 1 -> Framebuffers
//   DMA_CPU_TO_VRAM == 0 -> Sprite RAM
// DMA_ENABLE == 1 -> Blitter Control Registers
pub(in crate::sdk) enum VideoDma {
    DmaFb(Framebuffers),
    DmaBlit(Blitter),
    DmaSprites(SpriteMem),
}

/// Manages exclusive access to video hardware.
///
/// The GameTank's video memory at `$4000-$7FFF` can be one of three things
/// depending on hardware flags. `DmaManager` ensures only one is active
/// at a time using Rust's ownership system.
///
/// Access video hardware through [`Console::dma`](crate::sdk::scr::Console::dma).
pub struct DmaManager {
    pub(in crate::sdk) video_dma: Option<VideoDma>,
}

impl DmaManager {
    pub(in crate::sdk) fn new(vdma: VideoDma) -> Self {
        Self {
            video_dma: Some(vdma),
        }
    }

    /// Get exclusive access to the blitter for drawing operations.
    ///
    /// Returns `None` if video hardware is currently in use by another guard.
    /// The returned [`BlitterGuard`] releases the blitter when dropped.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
    /// blitter.draw_square(&mut console.sc, 0, 0, 128, 128, !0);
    /// blitter.wait_blit();
    /// ```
    pub fn blitter(&mut self, sc: &mut SystemControl) -> Option<BlitterGuard> {
        let b = self.video_dma.take()?.blitter(sc);
        Some(BlitterGuard {
            dma_slot: &mut self.video_dma,
            inner: b,
        })
    }

    /// Get exclusive access to the framebuffers.
    ///
    /// Returns `None` if video hardware is currently in use.
    /// Use this to flip buffers (double buffering) or write pixels directly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(fb) = console.dma.framebuffers(&mut console.sc) {
    ///     fb.flip(&mut console.sc); // Swap display/draw buffers
    /// }
    /// ```
    pub fn framebuffers(&mut self, sc: &mut SystemControl) -> Option<FramebuffersGuard> {
        let fb = self.video_dma.take()?.framebuffers(sc);
        Some(FramebuffersGuard {
            dma_slot: &mut self.video_dma,
            inner: fb,
        })
    }

    /// Get exclusive access to sprite RAM.
    ///
    /// Returns `None` if video hardware is currently in use.
    /// Use this to load sprite/tile data that the blitter will copy to the framebuffer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(mut sm) = console.dma.sprite_mem(&mut console.sc) {
    ///     sm.bytes().copy_from_slice(&MY_SPRITE_DATA);
    /// }
    /// ```
    pub fn sprite_mem(&mut self, sc: &mut SystemControl) -> Option<SpriteMemGuard> {
        let sm = self.video_dma.take()?.sprite_mem(sc);
        Some(SpriteMemGuard {
            dma_slot: &mut self.video_dma,
            inner: sm,
        })
    }
}

impl VideoDma {
    #[inline(always)]
    fn framebuffers(self, sc: &mut SystemControl) -> Framebuffers {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers,
            VideoDma::DmaBlit(blitter) => blitter.framebuffers(sc),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.framebuffers(sc),
        }
    }

    #[inline(always)]
    fn blitter(self, sc: &mut SystemControl) -> Blitter {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.blitter(sc),
            VideoDma::DmaBlit(blitter) => blitter,
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.blitter(sc),
        }
    }

    #[inline(always)]
    fn sprite_mem(self, sc: &mut SystemControl) -> SpriteMem {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.sprite_mem(sc),
            VideoDma::DmaBlit(blitter) => blitter.sprite_mem(sc),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem,
        }
    }
}

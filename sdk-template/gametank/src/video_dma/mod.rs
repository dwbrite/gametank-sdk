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
//! let mut blitter = console.dma.blitter(&mut console.video_flags).unwrap();
//!
//! // Fill rectangles with solid colors
//! blitter.draw_square(x, y, width, height, !color);
//!
//! // Copy sprites from sprite RAM to screen
//! blitter.draw_sprite(src_x, src_y, dst_x, dst_y, w, h);
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
//! blitter.draw_sprite(0, 0, 0, 0, 128, 128);
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
//!     console.flip_framebuffers();
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
//! let mut sm = console.dma.sprite_mem(&mut console.video_flags).unwrap();
//! sm.bytes()[..my_sprites.len()].copy_from_slice(my_sprites);
//! ```
//!
//! Sprite RAM has 8 pages of 256×256 pixels each (512KB total).
//! Select the page with [`BankFlags`](super::scr::BankFlags).

pub mod blitter;
pub mod framebuffers;
pub mod spritemem;

use crate::{
    scr::VideoFlags,
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
#[repr(C)]
pub(crate) enum VideoDma {
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
/// Access video hardware through [`Console::dma`](crate::console::Console::dma).
#[repr(C)]
pub struct DmaManager {
    pub(crate) video_dma: Option<VideoDma>,
}

impl DmaManager {
    pub(crate) fn new(vdma: VideoDma) -> Self {
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
    /// let mut blitter = console.dma.blitter(&mut console.video_flags).unwrap();
    /// blitter.draw_square(0, 0, 128, 128, !0);
    /// blitter.wait_blit();
    /// ```
    pub fn blitter<'a>(&'a mut self, vf: &'a mut VideoFlags) -> Option<BlitterGuard<'a>> {
        let b = self.video_dma.take()?.blitter(vf);
        Some(BlitterGuard {
            dma_slot: &mut self.video_dma,
            video_flags: vf,
            inner: b,
        })
    }

    /// Get exclusive access to the framebuffers.
    ///
    /// Returns `None` if video hardware is currently in use.
    /// Use this to write pixels directly.
    ///
    /// For double buffering, use [`Console::flip_framebuffers`](crate::console::Console::flip_framebuffers).
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(mut fb) = console.dma.framebuffers(&mut console.video_flags) {
    ///     fb.bytes()[0] = 0xFF; // Write a pixel
    /// }
    /// ```
    pub fn framebuffers<'a>(&'a mut self, vf: &'a mut VideoFlags) -> Option<FramebuffersGuard<'a>> {
        let fb = self.video_dma.take()?.framebuffers(vf);
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
    /// if let Some(mut sm) = console.dma.sprite_mem(&mut console.video_flags) {
    ///     sm.bytes().copy_from_slice(&MY_SPRITE_DATA);
    /// }
    /// ```
    pub fn sprite_mem<'a>(&'a mut self, vf: &'a mut VideoFlags) -> Option<SpriteMemGuard<'a>> {
        let sm = self.video_dma.take()?.sprite_mem(vf);
        Some(SpriteMemGuard {
            dma_slot: &mut self.video_dma,
            inner: sm,
        })
    }
}

impl VideoDma {
    #[inline(always)]
    fn framebuffers(self, vf: &mut VideoFlags) -> Framebuffers {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers,
            VideoDma::DmaBlit(blitter) => blitter.framebuffers(vf),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.framebuffers(vf),
        }
    }

    #[inline(always)]
    fn blitter(self, vf: &mut VideoFlags) -> Blitter {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.blitter(vf),
            VideoDma::DmaBlit(blitter) => blitter,
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.blitter(vf),
        }
    }

    #[inline(always)]
    fn sprite_mem(self, vf: &mut VideoFlags) -> SpriteMem {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.sprite_mem(vf),
            VideoDma::DmaBlit(blitter) => blitter.sprite_mem(vf),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem,
        }
    }
}

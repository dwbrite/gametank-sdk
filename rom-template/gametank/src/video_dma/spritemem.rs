//! # Sprite RAM Access
//!
//! Sprite RAM is **512KB** of graphics storage organized as **8 pages of 256×256 pixels**
//! each (64KB per page). The blitter copies from sprite RAM to the framebuffer.
//!
//! ## Memory Layout
//!
//! The Banking Register (bits 0-2) selects which page is active for both
//! CPU access and blitter operations. Each page is 256×256 pixels, but the CPU
//! can only access one **128×128 quadrant** at a time through `$4000-$7FFF`:
//!
//! ```text
//! Sprite RAM page (256×256):
//! ┌───────────┬───────────┐
//! │ Quadrant 1│ Quadrant 2│  Y = 0-127
//! │ (0,0)     │ (128,0)   │
//! ├───────────┼───────────┤
//! │ Quadrant 3│ Quadrant 4│  Y = 128-255
//! │ (0,128)   │ (128,128) │
//! └───────────┴───────────┘
//!   X=0-127     X=128-255
//! ```
//!
//! Use [`BankFlags`](crate::scr::BankFlags) to select the page (0-7),
//! and [`BlitterGuard::set_vram_quad`](super::blitter::BlitterGuard::set_vram_quad)
//! to select the quadrant before loading sprites.
//!
//! ## Loading Sprites
//!
//! ```ignore
//! if let Some(mut sm) = console.dma.sprite_mem(&mut console.sc) {
//!     // Copy sprite data into the current quadrant (16KB max)
//!     sm.bytes()[0..sprite_data.len()].copy_from_slice(&sprite_data);
//! }
//! ```
//!
//! ## Blitter Access
//!
//! The blitter can read the **full 256×256 page** using GX/GY coordinates 0-255.
//! The CPU quadrant restriction only affects direct memory access, not blits.

use crate::{
    scr::VideoFlags,
    video_dma::{VideoDma, blitter::Blitter, framebuffers::Framebuffers},
};

/// Write video flags to the hardware register at $2007.
#[inline(always)]
fn write_video_flags(flags: VideoFlags) {
    unsafe {
        core::ptr::write_volatile(0x2007 as *mut u8, flags.bits());
    }
}

#[repr(C)]
pub(crate) struct SpriteMem;

impl SpriteMem {
    #[inline(always)]
    pub fn blitter(self, vf: &mut VideoFlags) -> Blitter {
        vf.insert(VideoFlags::DMA_ENABLE);
        write_video_flags(*vf);
        Blitter
    }

    #[inline(always)]
    pub fn framebuffers(self, vf: &mut VideoFlags) -> Framebuffers {
        // DMA_ENABLE is already false
        vf.insert(VideoFlags::DMA_CPU_TO_VRAM);
        write_video_flags(*vf);
        Framebuffers
    }
}

/// Exclusive access to sprite RAM.
///
/// Provides direct byte access to the current 16KB sprite page quadrant.
/// Use this to load sprite/tile graphics that the blitter will copy to the framebuffer.
///
/// Released back to [`DmaManager`](super::DmaManager) when dropped.
pub struct SpriteMemGuard<'a> {
    pub(crate) dma_slot: &'a mut Option<VideoDma>,
    #[allow(dead_code)]
    pub(crate) inner: SpriteMem,
}

impl<'a> Drop for SpriteMemGuard<'a> {
    fn drop(&mut self) {
        *self.dma_slot = Some(VideoDma::DmaSprites(SpriteMem));
    }
}

impl<'a> SpriteMemGuard<'a> {
    /// Get a mutable reference to the 16KB sprite RAM quadrant.
    ///
    /// The current page is selected by [`BankFlags`](crate::scr::BankFlags) bits 0-2.
    /// The quadrant within the page is determined by the blitter's GX/GY counters.
    #[inline(always)]
    pub fn bytes(&mut self) -> &mut [u8; 0x4000] {
        unsafe { &mut *(0x4000 as *mut [u8; 0x4000]) }
    }
}

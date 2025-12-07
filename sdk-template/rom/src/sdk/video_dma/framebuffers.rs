//! # Framebuffer Access
//!
//! The GameTank has two 128×128 pixel framebuffers. While one is displayed,
//! the other can be drawn to — this is **double buffering**.
//!
//! ## Double Buffering
//!
//! Call [`FramebuffersGuard::flip`] each frame to swap which buffer is displayed:
//!
//! ```ignore
//! loop {
//!     unsafe { wait(); } // Wait for vblank
//!     
//!     // Swap buffers: the one we drew to is now displayed,
//!     // and we'll draw to the previously-displayed one
//!     if let Some(fb) = console.dma.framebuffers(&mut console.sc) {
//!         fb.flip(&mut console.sc);
//!     }
//!     
//!     // Now draw to the back buffer...
//! }
//! ```
//!
//! ## Direct Pixel Access
//!
//! Use [`FramebuffersGuard::bytes`] for direct access to the 16KB framebuffer:
//!
//! ```ignore
//! if let Some(mut fb) = console.dma.framebuffers(&mut console.sc) {
//!     let pixels = fb.bytes();
//!     pixels[0] = 0xFF; // Top-left pixel
//!     // pixels[y * 128 + x] = color;
//! }
//! ```
//!
//! The framebuffer is row-major, 128 bytes per row.

use crate::sdk::{
    scr::{BankFlags, SystemControl, VideoFlags},
    video_dma::{Blitter, VideoDma, spritemem::SpriteMem},
};

pub(in crate::sdk) struct Framebuffers;

impl Framebuffers {
    #[inline(always)]
    pub fn blitter(self, sc: &mut SystemControl) -> Blitter {
        sc.mir.video_reg.insert(VideoFlags::DMA_ENABLE);
        sc.scr.video_reg = sc.mir.video_reg;
        Blitter
    }

    #[inline(always)]
    pub fn sprite_mem(self, sc: &mut SystemControl) -> SpriteMem {
        // DMA_ENABLE is already false
        sc.mir.video_reg.remove(VideoFlags::DMA_CPU_TO_VRAM);
        sc.scr.video_reg = sc.mir.video_reg;
        SpriteMem
    }
}

/// Exclusive access to framebuffer memory.
///
/// Provides double buffering (`flip`) and direct pixel access (`bytes`).
/// Released back to [`DmaManager`](super::DmaManager) when dropped.
pub struct FramebuffersGuard<'a> {
    pub(in crate::sdk) dma_slot: &'a mut Option<VideoDma>,
    pub(in crate::sdk) inner: Framebuffers,
}

impl<'a> Drop for FramebuffersGuard<'a> {
    fn drop(&mut self) {
        *self.dma_slot = Some(VideoDma::DmaFb(Framebuffers));
    }
}

impl<'a> FramebuffersGuard<'a> {
    /// Get a mutable reference to the 16KB framebuffer.
    ///
    /// The framebuffer is 128×128 pixels in row-major order.
    /// Access pixel at (x, y) with `bytes[y * 128 + x]`.
    ///
    /// **Note:** About 100 horizontal lines are visible on a typical TV,
    /// and the rightmost column determines the border color.
    #[inline(always)]
    pub fn bytes(&mut self) -> &mut [u8; 0x4000] {
        unsafe { &mut *(0x4000 as *mut [u8; 0x4000]) }
    }

    /// Swap the displayed and drawing framebuffers (double buffering).
    ///
    /// After calling this, the buffer you were drawing to is now displayed,
    /// and you'll be drawing to the previously-displayed buffer.
    ///
    /// Call this once per frame, typically right after [`wait()`](crate::boot::wait).
    /// aliasing rules mean we can't borrow bytes and flip at the "same" time - I think?
    /// TODO: maybe flip returns a different framebufferguard, by consuming and returning?
    #[inline(always)]
    pub fn flip(self, sc: &mut SystemControl) -> Self {
        unsafe {
            sc.mir.banking.toggle(BankFlags::FRAMEBUFFER_SELECT);
            sc.mir.video_reg.toggle(VideoFlags::DMA_PAGE_OUT);
            sc.scr.banking = sc.mir.banking;
            sc.scr.video_reg = sc.mir.video_reg;
        }
        self
    }
}

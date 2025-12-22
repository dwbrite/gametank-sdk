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

use crate::{
    scr::VideoFlags,
    video_dma::{Blitter, VideoDma, spritemem::SpriteMem},
};

/// Write video flags to the hardware register at $2007.
#[inline(always)]
fn write_video_flags(flags: VideoFlags) {
    unsafe {
        core::ptr::write_volatile(0x2007 as *mut u8, flags.bits());
    }
}

#[repr(C)]
pub(crate) struct Framebuffers;

impl Framebuffers {
    #[inline(always)]
    pub fn blitter(self, vf: &mut VideoFlags) -> Blitter {
        vf.insert(VideoFlags::DMA_ENABLE);
        write_video_flags(*vf);
        Blitter
    }

    #[inline(always)]
    pub fn sprite_mem(self, vf: &mut VideoFlags) -> SpriteMem {
        // DMA_ENABLE is already false
        vf.remove(VideoFlags::DMA_CPU_TO_VRAM);
        write_video_flags(*vf);
        SpriteMem
    }
}

/// Exclusive access to framebuffer memory.
///
/// Provides direct pixel access (`bytes`).
/// For double buffering, use [`Console::flip_framebuffers`](crate::console::Console::flip_framebuffers).
/// Released back to [`DmaManager`](super::DmaManager) when dropped.
pub struct FramebuffersGuard<'a> {
    pub(crate) dma_slot: &'a mut Option<VideoDma>,
    #[allow(dead_code)]
    pub(crate) inner: Framebuffers,
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
}

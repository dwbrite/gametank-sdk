//! # Blitter
//!
//! The blitter draws rectangles and sprites to the screen. It runs **in parallel**
//! with the CPU, so you get free performance by doing CPU work during blits.
//!
//! ## Basic Drawing
//!
//! ```ignore
//! let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//!
//! // Fill a rectangle with a color (remember to invert with !)
//! blitter.draw_square(&mut console.sc, x, y, width, height, !color);
//! blitter.wait_blit();
//!
//! // Copy a sprite from sprite RAM to the screen
//! blitter.draw_sprite(&mut console.sc, src_x, src_y, dst_x, dst_y, width, height);
//! blitter.wait_blit();
//! ```
//!
//! ## Parallel Execution
//!
//! The blitter is a separate piece of hardware. While it's drawing, your CPU
//! is free to do other work:
//!
//! ```ignore
//! // Start a large blit (this returns immediately!)
//! blitter.draw_sprite(&mut console.sc, 0, 0, 0, 0, 128, 128);
//!
//! // All of this runs WHILE the blitter draws - essentially free!
//! update_physics();
//! process_input();
//! update_animations();
//! play_sound_effects();
//!
//! // Now wait for it to finish before the next blit
//! blitter.wait_blit();
//! ```
//!
//! The blitter draws ~60,000 pixels per frame at 60Hz. A full-screen background
//! (128×128 = 16K pixels) takes about 1/4 of a frame, giving you lots of time
//! for game logic.
//!
//! ## Colors
//!
//! Colors are 8-bit HSL format: `0bHHH_SS_LLL`
//!
//! ```ignore
//! const BLACK: u8  = 0b000_00_000;
//! const WHITE: u8  = 0b000_00_111;
//! const RED: u8    = 0b010_11_100;  // Hue=2, Sat=3, Lum=4
//! const GREEN: u8  = 0b111_11_100;  // Hue=7
//! const BLUE: u8   = 0b101_11_100;  // Hue=5
//!
//! // ALWAYS invert when drawing!
//! blitter.draw_square(10, 10, 32, 32, !RED);
//! ```

use crate::{
    boot::wait,
    blitter::{Bcr, SpriteQuadrant},
    scr::VideoFlags,
    video_dma::{framebuffers::Framebuffers, spritemem::SpriteMem, VideoDma},
};

#[repr(C)]
pub(crate) struct Blitter;

impl Blitter {
    #[inline(always)]
    pub fn framebuffers(self, vf: &mut VideoFlags) -> Framebuffers {
        vf.remove(VideoFlags::DMA_ENABLE);
        vf.insert(VideoFlags::DMA_CPU_TO_VRAM);
        write_video_flags(*vf);
        Framebuffers
    }

    #[inline(always)]
    pub fn sprite_mem(self, vf: &mut VideoFlags) -> SpriteMem {
        vf.remove(VideoFlags::DMA_ENABLE | VideoFlags::DMA_CPU_TO_VRAM);
        write_video_flags(*vf);
        SpriteMem
    }
}

/// Write video flags to the hardware register at $2007.
#[inline(always)]
fn write_video_flags(flags: VideoFlags) {
    unsafe {
        core::ptr::write_volatile(0x2007 as *mut u8, flags.bits());
    }
}

/// Exclusive access to the blitter hardware.
///
/// While you hold a `BlitterGuard`, you can perform drawing operations.
/// When dropped, the blitter is released back to the [`DmaManager`](super::DmaManager).
///
/// # Example
///
/// ```ignore
/// let mut blitter = console.dma.blitter(&mut console.video_flags).unwrap();
/// blitter.draw_square(10, 10, 32, 32, !0b111_00_000);
/// blitter.wait_blit();
/// // blitter is automatically released when it goes out of scope
/// ```
pub struct BlitterGuard<'a> {
    pub(crate) dma_slot: &'a mut Option<VideoDma>,
    pub(crate) video_flags: &'a mut VideoFlags,
    #[allow(dead_code)]
    pub(crate) inner: Blitter,
}

impl<'a> Drop for BlitterGuard<'a> {
    fn drop(&mut self) {
        *self.dma_slot = Some(VideoDma::DmaBlit(Blitter));
    }
}

impl<'a> BlitterGuard<'a> {
    /// Fill a rectangle with a solid color.
    ///
    /// # Arguments
    ///
    /// * `x` - Framebuffer X coordinate (0-127)
    /// * `y` - Framebuffer Y coordinate (0-127)
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `color` - Fill color (inverted GBR332 - use `!color`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Draw a red 16x16 square at (10, 20)
    /// blitter.draw_square(10, 20, 16, 16, !0b000_00_111);
    /// blitter.wait_blit();
    /// ```
    #[inline(always)]
    pub fn draw_square(
        &mut self,
        x: u8,
        y: u8,
        width: u8,
        height: u8,
        color: u8,
    ) {
        self.video_flags.insert(VideoFlags::DMA_COLORFILL);
        write_video_flags(*self.video_flags);
        unsafe {
            let bcr = Bcr::new();
            bcr.fb_x.write(x);
            bcr.fb_y.write(y);
            bcr.width.write(width);
            bcr.height.write(height);
            bcr.color.write(color);
            bcr.start.write(1);
        }
    }

    /// Copy a rectangular region from sprite RAM to the framebuffer.
    ///
    /// # Arguments
    ///
    /// * `sx` - Sprite RAM source X coordinate
    /// * `sy` - Sprite RAM source Y coordinate
    /// * `fb_x` - Framebuffer destination X (0-127)
    /// * `fb_y` - Framebuffer destination Y (0-127)
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Copy a 32x32 sprite from (0,0) in sprite RAM to (50,50) on screen
    /// blitter.draw_sprite(0, 0, 50, 50, 32, 32);
    /// blitter.wait_blit();
    /// ```
    #[inline(always)]
    pub fn draw_sprite(
        &mut self,
        sx: u8,
        sy: u8,
        fb_x: u8,
        fb_y: u8,
        width: u8,
        height: u8,
    ) {
        self.video_flags.remove(VideoFlags::DMA_COLORFILL);
        write_video_flags(*self.video_flags);
        unsafe {
            let bcr = Bcr::new();
            bcr.vram_x.write(sx);
            bcr.vram_y.write(sy);
            bcr.fb_x.write(fb_x);
            bcr.fb_y.write(fb_y);
            bcr.width.write(width);
            bcr.height.write(height);
            bcr.start.write(1);
        }
    }

    /// Set the sprite RAM quadrant for subsequent operations.
    ///
    /// Sprite RAM is organized as 256×512 pixels. This selects which
    /// 128×128 quadrant to use as the base for sprite coordinates.
    #[inline(always)]
    pub fn set_vram_quad(&mut self, quad: SpriteQuadrant) {
        unsafe {
            let bcr = &mut *(0x4000 as *mut Bcr);
            bcr.vram_x.write(quad.value_gx());
            bcr.vram_y.write(quad.value_gy());
            bcr.width.write(1);
            bcr.height.write(1);
            bcr.start.write(1);
            bcr.start.write(0);
        }
    }

    /// Wait for the current blit operation to complete.
    ///
    /// **Must be called** after each draw operation before starting another,
    /// or before accessing video memory directly.
    ///
    /// This waits for vblank (when the blitter finishes) and acknowledges
    /// the completion by writing 0 to the start register.
    #[inline(always)]
    pub fn wait_blit(&self) {
        unsafe {
            wait();
            let bcr = Bcr::new();
            bcr.start.write(0);
        }
    }

    /// Get direct access to the Blitter Control Registers.
    ///
    /// For advanced use cases where you need low-level control.
    pub fn bcr(&mut self) -> &mut Bcr {
        unsafe { Bcr::new() }
    }

    /// Draw letterbox borders to mask overscan areas.
    ///
    /// Draws black bars on:
    /// - Top 10 pixels (y: 0-9)
    /// - Bottom 10 pixels (y: 118-127)
    /// - Right column (x: 127, full height)
    ///
    /// This is intended to be called just before vsync to hide content
    /// in the overscan region that may not be visible on all displays.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Draw your scene...
    /// blitter.draw_sprite(0, 0, 0, 0, 127, 127);
    /// blitter.wait_blit();
    ///
    /// // Apply letterbox before vsync
    /// blitter.draw_letterbox();
    /// blitter.wait_blit();
    /// ```
    #[inline(always)]
    pub fn draw_letterbox(&mut self) {
        const BLACK: u8 = !0u8; // Inverted color: !0 = 0xFF = black
        const LETTERBOX_HEIGHT: u8 = 10;

        // Top bar: 127px wide, 10px tall, at (0, 0)
        self.draw_square(0, 0, 127, LETTERBOX_HEIGHT, BLACK);
        self.wait_blit();

        // Top bar: remaining 1px column at (127, 0)
        self.draw_square(127, 0, 1, LETTERBOX_HEIGHT, BLACK);
        self.wait_blit();

        // Bottom bar: 127px wide, 10px tall, at (0, 118)
        self.draw_square(0, 128 - LETTERBOX_HEIGHT, 127, LETTERBOX_HEIGHT, BLACK);
        self.wait_blit();

        // Bottom bar: remaining 1px column at (127, 118)
        self.draw_square(127, 128 - LETTERBOX_HEIGHT, 1, LETTERBOX_HEIGHT, BLACK);
        self.wait_blit();

        // Right column: 1px wide, middle section (between letterbox bars)
        // From y=10 to y=117 (108 pixels)
        self.draw_square(127, LETTERBOX_HEIGHT, 1, 128 - (LETTERBOX_HEIGHT * 2), BLACK);
    }
}

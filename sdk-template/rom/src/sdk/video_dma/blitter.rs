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
//! blitter.draw_square(&mut console.sc, 10, 10, 32, 32, !RED);
//! ```

use crate::{
    boot::wait,
    sdk::{
        blitter::{Bcr, BlitterFillMode, SpriteQuadrant},
        scr::{SystemControl, VideoFlags},
        video_dma::{framebuffers::Framebuffers, spritemem::SpriteMem, VideoDma},
    },
};

pub(in crate::sdk) struct Blitter;

impl Blitter {
    #[inline(always)]
    pub fn framebuffers(self, sc: &mut SystemControl) -> Framebuffers {
        sc.mir.video_reg.remove(VideoFlags::DMA_ENABLE);
        sc.mir.video_reg.insert(VideoFlags::DMA_CPU_TO_VRAM);
        sc.scr.video_reg = sc.mir.video_reg;
        Framebuffers
    }

    #[inline(always)]
    pub fn sprite_mem(self, sc: &mut SystemControl) -> SpriteMem {
        sc.mir
            .video_reg
            .remove(VideoFlags::DMA_ENABLE | VideoFlags::DMA_CPU_TO_VRAM);
        sc.scr.video_reg = sc.mir.video_reg;
        SpriteMem
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
/// let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
/// blitter.draw_square(&mut console.sc, 10, 10, 32, 32, !0b111_00_000);
/// blitter.wait_blit();
/// // blitter is automatically released when it goes out of scope
/// ```
pub struct BlitterGuard<'a> {
    pub(in crate::sdk) dma_slot: &'a mut Option<VideoDma>,
    pub(in crate::sdk) inner: Blitter,
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
    /// * `sc` - System control reference
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
    /// blitter.draw_square(&mut console.sc, 10, 20, 16, 16, !0b000_00_111);
    /// blitter.wait_blit();
    /// ```
    #[inline(always)]
    pub fn draw_square(
        &mut self,
        sc: &mut SystemControl,
        x: u8,
        y: u8,
        width: u8,
        height: u8,
        color: u8,
    ) {
        sc.set_fill_mode(BlitterFillMode::Color);
        unsafe {
            let mut bcr = Bcr::new();
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
    /// * `sc` - System control reference
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
    /// blitter.draw_sprite(&mut console.sc, 0, 0, 50, 50, 32, 32);
    /// blitter.wait_blit();
    /// ```
    #[inline(always)]
    pub fn draw_sprite(
        &mut self,
        sc: &mut SystemControl,
        sx: u8,
        sy: u8,
        fb_x: u8,
        fb_y: u8,
        width: u8,
        height: u8,
    ) {
        sc.set_fill_mode(BlitterFillMode::Sprite);
        unsafe {
            let mut bcr = Bcr::new();
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
            let mut bcr = Bcr::new();
            bcr.start.write(0);
        }
    }

    /// Get direct access to the Blitter Control Registers.
    ///
    /// For advanced use cases where you need low-level control.
    pub fn bcr(&mut self) -> &mut Bcr {
        unsafe { Bcr::new() }
    }
}

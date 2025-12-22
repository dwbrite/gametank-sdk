//! # System Control Register Flags
//!
//! This module defines the bitflags for the GameTank's system control registers.
//!
//! ## VideoFlags (`$2007`)
//!
//! Controls the blitter and video output:
//!
//! | Flag              | Effect                                           |
//! |-------------------|--------------------------------------------------|
//! | `DMA_ENABLE`      | Blitter active (1) vs CPU video access (0)       |
//! | `DMA_PAGE_OUT`    | Which framebuffer goes to the TV                 |
//! | `DMA_COLORFILL`   | Fill with color (1) vs copy sprites (0)          |
//! | `DMA_OPAQUE`      | Draw all pixels (1) vs skip color 0 (0)          |
//! | `DMA_GCARRY`      | Allow sprites > 16×16 (usually on)               |
//!
//! ## BankFlags (`$2005`)
//!
//! Controls sprite RAM page, framebuffer selection, and clipping.

bitflags::bitflags! {
    /// Video/Blitter control flags at `$2007`.
    ///
    /// These flags control the blitter's behavior and video output mode.
    /// Most are managed automatically by the SDK's DMA system.
    #[derive(Copy, Clone)]
    pub struct VideoFlags: u8 {
        /// Enable blitter DMA. When set, `$4000-$7FFF` maps to blitter registers.
        /// When clear, CPU can access video memory directly.
        const DMA_ENABLE           = 0b0000_0001;
        /// Select which framebuffer is displayed on screen.
        /// Toggle this each frame for double buffering.
        const DMA_PAGE_OUT        = 0b0000_0010;
        /// Enable NMI interrupt on vertical blank.
        const DMA_NMI             = 0b0000_0100;
        /// Blitter fill mode: set for color fill, clear for sprite copy.
        const DMA_COLORFILL       = 0b0000_1000;
        /// Graphics carry - enables smooth scrolling across sprite boundaries.
        const DMA_GCARRY          = 0b0001_0000;
        /// CPU video access mode: set for framebuffer, clear for sprite RAM.
        const DMA_CPU_TO_VRAM     = 0b0010_0000;
        /// Enable IRQ interrupt when blitter completes.
        const DMA_IRQ             = 0b0100_0000;
        /// Sprite transparency: set for opaque, clear to treat color 0 as transparent.
        const DMA_OPAQUE          = 0b1000_0000;
    }

    /// Banking control flags at `$2005`.
    ///
    /// Controls sprite RAM page selection, framebuffer access, and clipping.
    #[derive(Copy, Clone)]
    pub struct BankFlags: u8 {
        // Bits 0-2: Sprite RAM page (0–7)
        const SPRITE_PAGE_0       = 0b0000_0000;
        const SPRITE_PAGE_1       = 0b0000_0001;
        const SPRITE_PAGE_2       = 0b0000_0010;
        const SPRITE_PAGE_3       = 0b0000_0011;
        const SPRITE_PAGE_4       = 0b0000_0100;
        const SPRITE_PAGE_5       = 0b0000_0101;
        const SPRITE_PAGE_6       = 0b0000_0110;
        const SPRITE_PAGE_7       = 0b0000_0111;

        // Bit 3: Framebuffer select
        const FRAMEBUFFER_SELECT  = 0b0000_1000;

        // Bit 4: Clip L/R
        const CLIP_X              = 0b0001_0000;

        // Bit 5: Clip T/B
        const CLIP_Y              = 0b0010_0000;

        // Bits 6-7: RAM bank select
        const RAM_BANK_0          = 0b0000_0000;
        const RAM_BANK_1          = 0b0100_0000;
        const RAM_BANK_2          = 0b1000_0000;
        const RAM_BANK_3          = 0b1100_0000;
    }
}

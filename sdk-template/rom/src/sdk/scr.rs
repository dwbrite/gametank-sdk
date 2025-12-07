//! # Console & System Control
//!
//! The [`Console`] struct is your main interface to the GameTank hardware.
//! Your `main` function receives one, fully initialized:
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! fn main(mut console: Console) {
//!     // console.dma   - draw graphics (blitter, framebuffers, sprites)
//!     // console.sc    - system control (audio enable, etc.)
//!     // console.audio - 4KB audio RAM for synth data
//! }
//! ```
//!
//! ## What's in Console?
//!
//! | Field | Type | Use For |
//! |-------|------|---------|
//! | `dma` | [`DmaManager`] | Drawing! Get blitter, flip framebuffers, load sprites |
//! | `sc` | [`SystemControl`] | Audio enable, fill mode (rarely used directly) |
//! | `audio` | `&mut [u8; 4096]` | Audio RAM - copy firmware here, then voice data |
//!
//! ## Common Operations
//!
//! ```ignore
//! // Drawing
//! let mut blitter = console.dma.blitter(&mut console.sc).unwrap();
//! blitter.draw_square(&mut console.sc, x, y, w, h, !color);
//! blitter.wait_blit();
//!
//! // Double buffering
//! console.dma.framebuffers(&mut console.sc).unwrap().flip(&mut console.sc);
//!
//! // Audio setup
//! console.audio.copy_from_slice(&audio::FIRMWARE);
//! console.sc.set_audio(0xFF);  // Enable at ~14kHz
//! ```
//!
//! ## Advanced: Video Flags
//!
//! These are managed automatically by the SDK, but here's what they do:
//!
//! | Flag              | Effect                                           |
//! |-------------------|--------------------------------------------------|
//! | `DMA_ENABLE`      | Blitter active (1) vs CPU video access (0)       |
//! | `DMA_PAGE_OUT`    | Which framebuffer goes to the TV                 |
//! | `DMA_COLORFILL`   | Fill with color (1) vs copy sprites (0)          |
//! | `DMA_OPAQUE`      | Draw all pixels (1) vs skip color 0 (0)          |
//! | `DMA_GCARRY`      | Allow sprites > 16×16 (usually on)               |

use bit_field::BitField;
use bitflags::Bits;
use bitflags::{self, Flags};
use volatile_register::WO;

use crate::boot::{_VECTOR_TABLE, disable_irq_handler, enable_irq_handler, wait};
use crate::sdk::blitter::BlitterFillMode;
use crate::sdk::scr;
use crate::sdk::video_dma::framebuffers::Framebuffers;
use crate::sdk::video_dma::spritemem::SpriteMem;
use crate::sdk::video_dma::{DmaManager, VideoDma};

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

/// System Control Register hardware layout at `$2000-$2007`.
///
/// | Address | Name        | Description                              |
/// |---------|-------------|------------------------------------------|
/// | `$2000` | audio_reset | Write 1 to reset Audio Coprocessor       |
/// | `$2001` | audio_nmi   | Write 1 to send NMI to Audio Coprocessor |
/// | `$2005` | banking     | Banking control ([`BankFlags`])          |
/// | `$2006` | audio_reg   | Audio enable and sample rate             |
/// | `$2007` | video_reg   | Video/Blitter flags ([`VideoFlags`])     |
///
/// You typically don't access this directly; use [`SystemControl`] instead.
#[repr(C, packed)]
pub struct Scr {
    /// Write 1 to reset the Audio Coprocessor.
    pub audio_reset: u8,
    /// Write 1 to trigger an NMI on the Audio Coprocessor.
    pub audio_nmi: u8,
    _pad0: [u8; 3], // Skips to $2005
    /// Banking control register.
    pub banking: BankFlags,
    /// Audio enable and sample rate. Write `0xFF` for ~14kHz playback.
    pub audio_reg: u8,
    /// Video and blitter control flags.
    pub video_reg: VideoFlags,
}

/// Mirror of SCR in zero page for fast read-modify-write operations.
#[used]
#[unsafe(link_section = ".data.zp")]
pub static mut SCR_MIR: Scr = Scr {
    audio_reset: 69,
    audio_nmi: 0,
    _pad0: [0; 3],
    banking: BankFlags::empty(),
    audio_reg: 0b0_0000000,
    video_reg: VideoFlags::empty(),
};

/// Safe wrapper around the System Control Register.
///
/// Manages the hardware SCR at `$2000` along with a zero-page mirror for
/// efficient read-modify-write operations. The SDK handles keeping these in sync.
///
/// Most operations go through [`Console`] rather than accessing this directly.
pub struct SystemControl {
    pub(in crate::sdk) scr: &'static mut Scr,
    pub(in crate::sdk) mir: &'static mut Scr,
}

impl SystemControl {
    /// Initialize the System Control Register with default settings.
    ///
    /// Sets up sane defaults: NMI enabled, IRQ enabled, graphics carry on,
    /// opaque sprites, and double buffering ready.
    pub fn init() -> Self {
        unsafe {
            // mir is zeroe'd
            let mir = &mut SCR_MIR;
            let scr = &mut *(0x2000 as *mut Scr);

            mir.video_reg.insert(VideoFlags::DMA_NMI);
            mir.video_reg.insert(VideoFlags::DMA_IRQ);
            mir.video_reg.insert(VideoFlags::DMA_GCARRY);
            mir.video_reg.insert(VideoFlags::DMA_OPAQUE);
            mir.video_reg.insert(VideoFlags::DMA_PAGE_OUT);
            mir.banking.remove(BankFlags::FRAMEBUFFER_SELECT);

            scr.audio_reset = mir.audio_reset;
            scr.audio_nmi = mir.audio_nmi;
            scr.banking = mir.banking;
            scr.audio_reg = mir.audio_reg;
            scr.video_reg = mir.video_reg;

            Self { scr, mir }
        }
    }

    /// Set the blitter fill mode.
    ///
    /// - [`BlitterFillMode::Color`] - Fill rectangles with a solid color
    /// - [`BlitterFillMode::Sprite`] - Copy from sprite RAM to framebuffer
    #[inline(always)]
    pub fn set_fill_mode(&mut self, mode: BlitterFillMode) {
        self.mir
            .video_reg
            .set(VideoFlags::DMA_COLORFILL, mode == BlitterFillMode::Color);
        self.scr.video_reg = self.mir.video_reg;
    }

    /// Enable audio and set sample rate.
    ///
    /// Write `0xFF` to enable audio at ~14kHz (the standard rate).
    /// Write `0x00` to disable audio.
    pub fn set_audio(&mut self, value: u8) {
        self.scr.audio_reg = value;
    }
}


/// The main entry point for GameTank programs.
///
/// `Console` bundles together all the hardware interfaces you need:
/// - [`sc`](Console::sc) - System control for video/audio registers
/// - [`dma`](Console::dma) - DMA manager for blitter and video memory access
/// - [`audio`](Console::audio) - Direct access to the 4KB audio RAM at `$3000`
///
/// # Example
///
/// ```ignore
/// #[unsafe(no_mangle)]
/// fn main(mut console: Console) {
///     // Load audio firmware
///     console.audio.copy_from_slice(FIRMWARE);
///     console.sc.set_audio(0xFF); // Enable at 14kHz
///     
///     loop {
///         unsafe { wait(); } // Wait for vblank
///         
///         // Flip framebuffers for double buffering
///         if let Some(fb) = console.dma.framebuffers(&mut console.sc) {
///             fb.flip(&mut console.sc);
///         }
///         
///         // Draw with the blitter
///         if let Some(mut blitter) = console.dma.blitter(&mut console.sc) {
///             blitter.draw_square(&mut console.sc, 10, 10, 16, 16, !0b111_00_000);
///             blitter.wait_blit();
///         }
///     }
/// }
/// ```
pub struct Console {
    /// System control - access to video flags, audio, and banking registers.
    pub sc: SystemControl,
    /// DMA manager - provides exclusive access to blitter, framebuffers, or sprite RAM.
    pub dma: DmaManager,
    /// Audio RAM - 4KB shared with the Audio Coprocessor at `$3000-$3FFF`.
    /// Copy your audio firmware here, then use it for voice/instrument data.
    pub audio: &'static mut [u8; 4096],
}

impl Console {
    /// Initialize the console hardware.
    ///
    /// Called automatically by the boot code before `main()`.
    pub fn init() -> Self {
        // TODO: singleton-ize this?
        Self {
            sc: SystemControl::init(),
            dma: DmaManager::new(VideoDma::DmaSprites(SpriteMem)),
            audio: unsafe {&mut *(0x3000 as *mut [u8; 4096])},
        }
    }
}

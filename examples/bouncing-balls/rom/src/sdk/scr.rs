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
    #[derive(Copy, Clone)]
    pub struct VideoFlags: u8 {
        const DMA_ENABLE           = 0b0000_0001;
        const DMA_PAGE_OUT        = 0b0000_0010;
        const DMA_NMI             = 0b0000_0100;
        const DMA_COLORFILL       = 0b0000_1000;
        const DMA_GCARRY          = 0b0001_0000;
        const DMA_CPU_TO_VRAM     = 0b0010_0000;
        const DMA_IRQ             = 0b0100_0000;
        const DMA_OPAQUE          = 0b1000_0000;
    }

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

// #[repr(C, packed)]
// pub struct Bcr {
//     pub fb_x: WO<u8>,
//     pub fb_y: WO<u8>,
//     pub vram_x: WO<u8>,
//     pub vram_y: WO<u8>,
//     pub width: WO<u8>,
//     pub height: WO<u8>,
//     pub start: WO<u8>,
//     pub color: WO<u8>,
// }

/// System Control Register
/// $2000 	Write 1 to reset audio coprocessor
/// $2001 	Write 1 to send NMI to audio coprocessor
/// $2005 	Banking Register
/// $2006 	Audio enable and sample rate
/// $2007 	Video/Blitter Flags
#[repr(C, packed)]
pub struct Scr {
    pub audio_reset: u8,
    pub audio_nmi: u8,
    _pad0: [u8; 3], // Skips to $2005
    pub banking: BankFlags,
    pub audio_reg: u8,
    pub video_reg: VideoFlags,
}

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

pub struct SystemControl {
    pub(in crate::sdk) scr: &'static mut Scr,
    pub(in crate::sdk) mir: &'static mut Scr,
}

impl SystemControl {
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

    //
    #[inline(always)]
    pub fn set_fill_mode(&mut self, mode: BlitterFillMode) {
        self.mir
            .video_reg
            .set(VideoFlags::DMA_COLORFILL, mode == BlitterFillMode::Color);
        self.scr.video_reg = self.mir.video_reg;
    }

    pub fn set_audio(&mut self, value: u8) {
        self.scr.audio_reg = value;
    }
}



pub struct Console {
    pub sc: SystemControl,
    pub dma: DmaManager,
    pub audio: &'static mut [u8; 4096],
}

impl Console {
    pub fn init() -> Self {
        // TODO: singleton-ize this?
        Self {
            sc: SystemControl::init(),
            dma: DmaManager::new(VideoDma::DmaSprites(SpriteMem)),
            audio: unsafe {&mut *(0x3000 as *mut [u8; 4096])},
        }
    }
}

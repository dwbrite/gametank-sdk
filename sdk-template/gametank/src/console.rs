use crate::{input::GenesisGamepad, scr::{BankFlags, VideoFlags}, via::Via, video_dma::{DmaManager, VideoDma, blitter::BlitterGuard, spritemem::SpriteMem}};

/// Write-only register at $2005
const BANK_REG: *mut u8 = 0x2005 as *mut u8;
/// Write-only register at $2007
const VIDEO_REG: *mut u8 = 0x2007 as *mut u8;

pub struct AudioManager {
    pub aram: &'static mut [u8; 4096],
    pub audio_reset: &'static mut u8,
    pub audio_nmi: &'static mut u8,
    pub audio_freq: &'static mut u8,
}

impl AudioManager {
    
}



pub struct Console {
    /// Shadow copy of write-only bank register at $2005
    pub bank_flags: BankFlags,
    /// Shadow copy of write-only video register at $2007
    pub video_flags: VideoFlags,
    pub dma: DmaManager,
    pub audio: AudioManager,
    pub via: &'static mut Via,
}

impl Console {
    /// Initialize the console and store it in the ZP static.
    /// Returns a mutable reference to the initialized console.
    pub fn init() -> Console {
        let bank_flags = BankFlags::FRAMEBUFFER_SELECT;
        let mut video_flags = VideoFlags::empty();
        video_flags.insert(VideoFlags::DMA_NMI);
        video_flags.insert(VideoFlags::DMA_IRQ);
        video_flags.insert(VideoFlags::DMA_GCARRY);
        video_flags.insert(VideoFlags::DMA_OPAQUE);
        video_flags.insert(VideoFlags::DMA_PAGE_OUT);

        let console = Self {
            bank_flags,
            video_flags,
            dma: DmaManager::new(VideoDma::DmaSprites(SpriteMem)),
            audio: AudioManager {
                aram: unsafe { &mut *(0x3000 as *mut [u8; 4096]) },
                audio_reset: unsafe { &mut *(0x2000 as *mut u8) },
                audio_nmi: unsafe { &mut *(0x2001 as *mut u8) },
                audio_freq: unsafe { &mut *(0x2006 as *mut u8) },
            },
            via: unsafe { Via::new() },
        };
        console
    }

    /// Write the current bank_flags shadow to hardware.
    #[inline(always)]
    pub fn write_bank_flags(&self) {
        unsafe { core::ptr::write_volatile(BANK_REG, self.bank_flags.bits()); }
    }

    /// Write the current video_flags shadow to hardware.
    #[inline(always)]
    pub fn write_video_flags(&self) {
        unsafe { core::ptr::write_volatile(VIDEO_REG, self.video_flags.bits()); }
    }

    #[inline(always)]
    pub fn flip_framebuffers(&mut self) {
        self.bank_flags.toggle(BankFlags::FRAMEBUFFER_SELECT);
        self.video_flags.toggle(VideoFlags::DMA_PAGE_OUT);
        self.write_bank_flags();
        self.write_video_flags();
    }

    pub fn genesis_gamepads(&self) -> (GenesisGamepad<1>, GenesisGamepad<2>) {
        (GenesisGamepad::new(), GenesisGamepad::new())
    }

    pub fn set_rom_bank(&mut self, bank: u8) {
        self.via.change_rom_bank(bank);
    }

    pub fn blitter(&mut self) -> Option<BlitterGuard<'_>> {
        self.video_flags.set(VideoFlags::DMA_COLORFILL, false);
        self.dma.blitter(&mut self.video_flags)
    }
}
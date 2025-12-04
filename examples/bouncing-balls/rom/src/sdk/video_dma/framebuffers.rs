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
    #[inline(always)]
    pub fn bytes(&mut self) -> &mut [u8; 0x4000] {
        unsafe { &mut *(0x4000 as *mut [u8; 0x4000]) }
    }

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

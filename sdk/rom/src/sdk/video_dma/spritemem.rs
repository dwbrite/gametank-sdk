use crate::sdk::{
    scr::{SystemControl, VideoFlags},
    video_dma::{VideoDma, blitter::Blitter, framebuffers::Framebuffers},
};

pub(in crate::sdk) struct SpriteMem;

impl SpriteMem {
    #[inline(always)]
    pub fn blitter(self, sc: &mut SystemControl) -> Blitter {
        sc.mir.video_reg.insert(VideoFlags::DMA_ENABLE);
        sc.scr.video_reg = sc.mir.video_reg;
        Blitter
    }

    #[inline(always)]
    pub fn framebuffers(self, sc: &mut SystemControl) -> Framebuffers {
        // DMA_ENABLE is already false
        sc.mir.video_reg.insert(VideoFlags::DMA_CPU_TO_VRAM);
        sc.scr.video_reg = sc.mir.video_reg;
        Framebuffers
    }
}

pub struct SpriteMemGuard<'a> {
    pub(in crate::sdk) dma_slot: &'a mut Option<VideoDma>,
    pub(in crate::sdk) inner: SpriteMem,
}

impl<'a> Drop for SpriteMemGuard<'a> {
    fn drop(&mut self) {
        *self.dma_slot = Some(VideoDma::DmaSprites(SpriteMem));
    }
}

impl<'a> SpriteMemGuard<'a> {
    #[inline(always)]
    pub fn bytes(&mut self) -> &mut [u8; 0x4000] {
        unsafe { &mut *(0x4000 as *mut [u8; 0x4000]) }
    }
}

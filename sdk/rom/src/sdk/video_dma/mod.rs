pub mod blitter;
pub mod framebuffers;
pub mod spritemem;

use crate::sdk::{
    scr::{SystemControl, VideoFlags},
    video_dma::{
        blitter::{Blitter, BlitterGuard},
        framebuffers::{Framebuffers, FramebuffersGuard},
        spritemem::{SpriteMem, SpriteMemGuard},
    },
};

// DMA_ENABLE == 0 -> CPU can see video memory
//   DMA_CPU_TO_VRAM == 1 -> Framebuffers
//   DMA_CPU_TO_VRAM == 0 -> Sprite RAM
// DMA_ENABLE == 1 -> Blitter Control Registers
pub(in crate::sdk) enum VideoDma {
    DmaFb(Framebuffers),
    DmaBlit(Blitter),
    DmaSprites(SpriteMem),
}

pub struct DmaManager {
    pub(in crate::sdk) video_dma: Option<VideoDma>,
}

impl DmaManager {
    pub(in crate::sdk) fn new(vdma: VideoDma) -> Self {
        Self {
            video_dma: Some(vdma),
        }
    }

    pub fn blitter(&mut self, sc: &mut SystemControl) -> Option<BlitterGuard> {
        let b = self.video_dma.take()?.blitter(sc);
        Some(BlitterGuard {
            dma_slot: &mut self.video_dma,
            inner: b,
        })
    }

    pub fn framebuffers(&mut self, sc: &mut SystemControl) -> Option<FramebuffersGuard> {
        let fb = self.video_dma.take()?.framebuffers(sc);
        Some(FramebuffersGuard {
            dma_slot: &mut self.video_dma,
            inner: fb,
        })
    }

    pub fn sprite_mem(&mut self, sc: &mut SystemControl) -> Option<SpriteMemGuard> {
        let sm = self.video_dma.take()?.sprite_mem(sc);
        Some(SpriteMemGuard {
            dma_slot: &mut self.video_dma,
            inner: sm,
        })
    }
}

impl VideoDma {
    #[inline(always)]
    fn framebuffers(self, sc: &mut SystemControl) -> Framebuffers {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers,
            VideoDma::DmaBlit(blitter) => blitter.framebuffers(sc),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.framebuffers(sc),
        }
    }

    #[inline(always)]
    fn blitter(self, sc: &mut SystemControl) -> Blitter {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.blitter(sc),
            VideoDma::DmaBlit(blitter) => blitter,
            VideoDma::DmaSprites(sprite_mem) => sprite_mem.blitter(sc),
        }
    }

    #[inline(always)]
    fn sprite_mem(self, sc: &mut SystemControl) -> SpriteMem {
        match self {
            VideoDma::DmaFb(framebuffers) => framebuffers.sprite_mem(sc),
            VideoDma::DmaBlit(blitter) => blitter.sprite_mem(sc),
            VideoDma::DmaSprites(sprite_mem) => sprite_mem,
        }
    }
}

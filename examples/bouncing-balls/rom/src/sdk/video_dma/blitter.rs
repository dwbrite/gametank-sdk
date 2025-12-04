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

    #[inline(always)]
    pub fn wait_blit(&self) {
        unsafe {
            wait();
            let mut bcr = Bcr::new();
            bcr.start.write(0);
        }
    }

    pub fn bcr(&mut self) -> &mut Bcr {
        unsafe { Bcr::new() }
    }
}

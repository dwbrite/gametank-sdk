use alloc::boxed::Box;
use core::cell::RefCell;
use bitfield::bitfield;
bitfield!{
    pub struct BankingRegister(u8);
    impl Debug;
    pub vram_page, set_vram_page: 2, 0;
    pub framebuffer, set_framebuffer: 3;
    pub clip_blits_h, set_clip_blits_h: 4;
    pub clip_blits_v, set_clip_blits_v: 5;
    pub ram_bank, set_ram_bank: 7, 6;
}

bitfield!{
    pub struct BlitterFlags(u8);
    impl Debug;
    pub dma_enable, set_dma_enable : 0;
    pub dma_page_out, set_dma_page_out : 1;
    pub dma_nmi, set_dma_nmi : 2;
    pub dma_colorfill_enable, set_dma_colorfill_enable : 3;
    pub dma_gcarry, set_dma_gcarry : 4;
    pub dma_cpu_to_vram, set_dma_cpu_to_vram : 5;
    pub dma_irq, set_dma_irq : 6;
    pub dma_opaque, set_dma_opaque : 7;
}


#[derive(Debug)]
pub enum GraphicsMemoryMap {
    FrameBuffer,
    VRAM,
    BlitterRegisters
}


pub type FrameBuffer = Box<[u8; 128*128]>;
pub type SharedFrameBuffer = RefCell<FrameBuffer>;

pub fn new_framebuffer(fill: u8) -> SharedFrameBuffer {
    RefCell::new(Box::new([fill; 128*128]))
}

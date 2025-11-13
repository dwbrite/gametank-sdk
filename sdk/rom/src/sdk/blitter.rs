use volatile_register::WO;

use crate::{
    boot::{wait},
    sdk::{scr::SystemControl, video_dma::blitter::Blitter},
};

/// Blitter Control Registers
/// vram_VX 0x4000
/// vram_VY 0x4001
/// vram_GX 0x4002
/// vram_GY 0x4003
/// vram_WIDTH 0x4004
/// vram_HEIGHT 0x4005
/// vram_START 0x4006
/// vram_COLOR 0x4007
#[repr(C, packed)]
pub struct Bcr {
    pub fb_x: WO<u8>,
    pub fb_y: WO<u8>,
    pub vram_x: WO<u8>,
    pub vram_y: WO<u8>,
    pub width: WO<u8>,
    pub height: WO<u8>,
    pub start: WO<u8>,
    pub color: WO<u8>,
}

impl Bcr {
    #[inline(always)]
    pub(in crate::sdk) unsafe fn new() -> &'static mut Bcr {
        unsafe { &mut *(0x4000 as *mut Bcr) }
    }
}

#[derive(PartialEq)]
pub enum BlitterFillMode {
    Sprite,
    Color,
}

pub enum SpriteQuadrant {
    One,
    Two,
    Three,
    Four,
}

impl SpriteQuadrant {
    #[inline(always)]
    pub fn value_gx(&self) -> u8 {
        match self {
            Self::One | Self::Three => 0,
            Self::Two | Self::Four => 128,
        }
    }

    #[inline(always)]
    pub fn value_gy(&self) -> u8 {
        match self {
            Self::One | Self::Two => 0,
            Self::Three | Self::Four => 128,
        }
    }
}

use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use crate::cartridges::Cartridge;

#[derive(Debug, Clone)]
pub struct Cartridge16K {
    data: Box<[u8; 0x4000]>
}

impl Deref for Cartridge16K {
    type Target = [u8; 0x4000];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Cartridge16K {

    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Cartridge for Cartridge16K {
    fn from_slice(slice: &[u8]) -> Self {
        let mut data = [0; 0x4000];
        data[0..0x4000].copy_from_slice(&slice);
        Self {
            data: Box::new(data),
        }
    }

    #[inline(always)]
    fn read_byte(&self, address: u16) -> u8 {
        unsafe { *self.data.get_unchecked((address as usize) & 0x3FFF) }
    }
}

use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use crate::cartridges::Cartridge;

#[derive(Debug, Clone)]
pub struct Cartridge8K {
    data: Box<[u8; 0x2000]>
}

impl Deref for Cartridge8K {
    type Target = [u8; 0x2000];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Cartridge8K {

    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Cartridge for Cartridge8K {
    fn from_slice(slice: &[u8]) -> Self {
        let mut data = [0; 0x2000];
        data[0..0x2000].copy_from_slice(&slice);
        Self {
            data: Box::new(data),
        }
    }

    #[inline(always)]
    fn read_byte(&self, address: u16) -> u8 {
        unsafe { *self.data.get_unchecked((address - 0x6000) as usize) }
    }
}

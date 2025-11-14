#![no_std]
#![allow(clippy::disallowed_methods, clippy::single_match)]
#![allow(dead_code, unused_variables, unused_imports, internal_features, static_mut_refs)]
extern crate alloc;

use alloc::boxed::Box;
use log::{error};
use gte_w65c02s::{System, W65C02S};

pub mod audio_output;

pub static mut ARAM: &'static mut [u8; 0x1000]  = &mut [0; 0x1000];

#[derive(Default, Debug)]
pub struct AcpBus {
    pub irq_counter: i32,
    
    pub sample: u8,
}

impl AcpBus {
    #[inline(always)]
    pub(crate) fn write_byte(&mut self, address: u16, data: u8) {
        unsafe { *ARAM.get_unchecked_mut((address & 0x0FFF) as usize) = data; }
        match address {
            0x8000..=0xFFFF => {
                self.sample = data;
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        unsafe { *ARAM.get_unchecked((address as usize) & 0x0FFF) }
    }
}

impl System for AcpBus {
    #[inline(always)]
    fn read(&mut self, _: &mut W65C02S, addr: u16) -> u8 {
        self.read_byte(addr)
    }

    #[inline(always)]
    fn write(&mut self, _: &mut W65C02S, addr: u16, data: u8) {
        self.write_byte(addr, data);
    }
}
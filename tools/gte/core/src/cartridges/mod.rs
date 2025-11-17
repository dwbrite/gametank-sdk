#![allow(dead_code, unused_variables, unused_imports, internal_features, static_mut_refs)]

pub mod cart8k;
pub mod cart16k;
pub mod cart32k;
pub mod cart2mj21;

use alloc::boxed::Box;
use log::error;
use crate::cartridges::cart2mj21::Cartridge2M;
use crate::cartridges::cart8k::Cartridge8K;
use crate::cartridges::cart16k::Cartridge16K;
use crate::cartridges::cart32k::{Cartridge32K};

pub trait Cartridge {
    fn from_slice(slice: &[u8]) -> Self;
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, data: u8) {
        //default impl do nothing
    }
    fn update_via(&mut self, via: &mut [[u8; 16]; 2]) {
        //default impl do nothing
    }
}

#[derive(Debug, Clone)]
pub enum CartridgeType {
    Cart8k(Cartridge8K),
    Cart16k(Cartridge16K),
    Cart32k(Cartridge32K),
    Cart2m(Box<Cartridge2M>),
}

impl CartridgeType {
    pub fn from_slice(slice: &[u8]) -> Self {
        match slice.len() {
            0x2000 => {
                CartridgeType::Cart8k(Cartridge8K::from_slice(slice))
            }
            0x4000 => {
                CartridgeType::Cart16k(Cartridge16K::from_slice(slice))
            }
            0x8000 => {
                CartridgeType::Cart32k(Cartridge32K::from_slice(slice))
            }
            0x200000 => {
                CartridgeType::Cart2m(Box::new(Cartridge2M::from_slice(slice)))
            }
            _ => {
                panic!("unimplemented");
            }
        }
    }

    #[inline(always)]
    pub fn read_byte(&self, address: u16) -> u8 {
        match self {
            CartridgeType::Cart8k(c) => {c.read_byte(address)}
            CartridgeType::Cart16k(c) => {c.read_byte(address)}
            CartridgeType::Cart32k(c) => {c.read_byte(address)}
            CartridgeType::Cart2m(c) => {c.read_byte(address)}
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match self {
            CartridgeType::Cart2m(c) => { c.write_byte(address, data) }
            _ => { error!("attempted write to non-writable cartridge") }
        }
    }

    pub fn update_via(&mut self, via: &mut [[u8; 16]; 2]) {
        match self {
            CartridgeType::Cart2m(c) => { c.update_via(via) }
            _ => {}
        }
    }
}

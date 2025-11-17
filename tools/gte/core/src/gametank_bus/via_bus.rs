use crate::gametank_bus::reg_system_control::VIA_IORA;

pub const IORB: usize    = 0x0;
pub const IORA: usize    = 0x1;
pub const DDRB: usize   = 0x2;
pub const DDRA: usize   = 0x3;
pub const T1CL: usize   = 0x4;
pub const T1CH: usize   = 0x5;
pub const T1LL: usize   = 0x6;
pub const T1LH: usize   = 0x7;
pub const T2CL: usize   = 0x8;
pub const T2CH: usize   = 0x9;
pub const SR: usize     = 0xA;
pub const ACR: usize    = 0xB;
pub const PCR: usize    = 0xC;
pub const IFR: usize    = 0xD;
pub const IER: usize    = 0xE;
pub const ORA_NH: usize = 0xF;

// pub const SPI_BIT_CLK : u8 = 0b00000001;
// pub const SPI_BIT_MOSI: u8 = 0b00000010;
// pub const SPI_BIT_CS  : u8 = 0b00000100;
// pub const SPI_BIT_MISO: u8 = 0b10000000;

// pub struct Via {
//     registers: [u8; 16],

// }

// impl Via {
//     pub fn write_via_reg(&mut self, addr: usize, data: u8) {

//     }

//     pub fn get_bus(&mut self, address: u16) -> u8 {
//         match address {
//             0x5000..=0x5FFF => {
//                 0
//             }
//             _ => { panic!("how the hell did you get here?"); }
//         }
//     }
// }
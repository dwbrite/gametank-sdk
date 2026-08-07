use alloc::boxed::Box;
use core::cell::Ref;
use log::{debug, warn};
use gte_w65c02s::{System, W65C02S};
use crate::cartridges::cart2mj21::Cartridge2M;
use crate::cartridges::CartridgeType;
use crate::gametank_bus::reg_system_control::*;
use gte_acp::ARAM;
use crate::gametank_bus::cpu_bus::ByteDecorator::{AudioRam, CpuStack, SystemRam, Unreadable, Vram, ZeroPage};
use crate::gametank_bus::reg_blitter::{BlitStart, BlitterRegisters};
use crate::gametank_bus::reg_etc::{new_framebuffer, BankingRegister, BlitterFlags, FrameBuffer, GraphicsMemoryMap, SharedFrameBuffer};
use crate::gametank_bus::reg_system_control::*;
use crate::inputs::GamePad;

const CURRENT_GAME: &[u8] = &[0; 0x2000];

#[derive(Copy, Clone, Debug)]
pub enum ByteDecorator {
    ZeroPage(u8),
    CpuStack(u8),
    SystemRam(u8),
    // SCR(u8),
    // VersatileInterfaceAdapter(u8),
    AudioRam(u8),
    Vram(u8),
    Framebuffer(u8),
    // Blitter(u8),
    Aram(u8),
    Unreadable(u8),
}

#[derive(Debug)]
pub struct CpuBus {
    pub system_control: SystemControl,
    pub blitter: BlitterRegisters,

    // heap allocations to prevent stackoverflow, esp on web
    pub ram_banks: Box<[[u8; 0x2000]; 4]>,
    pub framebuffers: [SharedFrameBuffer; 2],
    pub vram_banks: Box<[[u8; 256*256]; 8]>,

    pub vram_quad_written: [bool; 32],

    // pub aram: Option<ARAM>,
    pub cartridge: CartridgeType,
}

impl Default for CpuBus {
    fn default() -> Self {
        let bus = Self {
            system_control: SystemControl {
                reset_acp: 0,
                nmi_acp: 0,
                banking_register: BankingRegister(0),
                via_regs: [0; 16],
                audio_enable_sample_rate: 0,
                dma_flags: BlitterFlags(0b0111_1111),
                gamepads: [GamePad::default(), GamePad::default()]
            },
            blitter: BlitterRegisters {
                vx: 0,
                vy: 0,
                gx: 0,
                gy: 0,
                width: 127,
                height: 127,
                start: BlitStart {
                    write: 0,
                    addressed: false,
                },
                color: 0b101_00_000, // offwhite
            },
            ram_banks: Box::new([[0; 0x2000]; 4]),
            framebuffers: [new_framebuffer(0x00), new_framebuffer(0xFF)],
            vram_banks: Box::new([[0; 256*256]; 8]),
            cartridge: CartridgeType::from_slice(CURRENT_GAME),
            // aram: Some(Box::new([0; 0x1000])),
            vram_quad_written: [false; 32],
        };

        bus
    }
}

impl CpuBus {
    pub fn read_full_framebuffer(&self) -> Ref<'_, FrameBuffer> {
        let fb = self.system_control.get_framebuffer_out();
        self.framebuffers[fb].borrow()
    }

    // fn update_flash_shift_register(&mut self, next_val: u8) {
    //     match &mut self.cartridge {
    //         CartridgeType::Cart2m(cartridge) => {
    //             // TODO: Care about DDR bits
    //             // For now, assuming that if we're using Flash2M hardware, we're behaving ourselves
    //             let old_val = self.system_control.via_regs[VIA_IORA]; // Get the previous value from the VIA
    //             let rising_bits = next_val & !old_val;

    //             if rising_bits & VIA_SPI_BIT_CLK != 0 {
    //                 cartridge.bank_shifter <<= 1; // Shift left
    //                 cartridge.bank_shifter &= 0xFE; // Ensure the last bit is cleared
    //                 cartridge.bank_shifter |= ((old_val & VIA_SPI_BIT_MOSI) != 0) as u8; // Set the last bit based on MOSI
    //             } else if rising_bits & VIA_SPI_BIT_CS != 0 {
    //                 // Flash cart CS is connected to latch clock
    //                 if (cartridge.bank_mask ^ cartridge.bank_shifter) & 0x80 != 0 {
    //                     // TODO: support saving
    //                     // self.save_nvram();
    //                     warn!("Saving is not yet supported");
    //                 }
    //                 cartridge.bank_mask = cartridge.bank_shifter; // Update the bank mask
                    
    //                 // Check if this is NOT a FLASH2M_RAM32K type
    //                 // For now, assuming Cart2m is the standard Flash2M type, not RAM32K variant
    //                 // If you need to distinguish, you may need to add a variant field to Cartridge2M
    //                 // cartridge.bank_mask |= 0x80; // Uncomment if this cart type should set bit 7
                    
    //                 debug!("Flash bank mask set to 0x{:x}", cartridge.bank_mask);
    //             }
    //         },
    //         _ => {} // do nothing
    //     }
    // }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            // system RAM
            0x0000..=0x1FFF => {
                self.ram_banks[self.system_control.get_ram_bank()][address as usize] = data;
                // println!("${:04X}={:02X}", address, data);
            }

            // system control registers
            0x2000..=0x2009 => {
                self.system_control.write_byte(address, data);
                // println!("${:04X}={:08b}", address, data);
            }

            // versatile interface adapter (GPIO, timers)
            0x2800..=0x280F => {
                // TODO: this is a bit hacky since the mutable via regs in "update_via" won't track changes after :/
                let before_reg = self.system_control.via_regs.clone();

                let register = (address & 0xF) as usize;
                self.system_control.via_regs[register] = data;

                self.cartridge.update_via(&mut [before_reg, self.system_control.via_regs]);
            }

            // audio RAM
            0x3000..=0x3FFF => unsafe {
                ARAM[(address - 0x3000) as usize] = data;
            }

            // VRAM/Framebuffer/Blitter
            0x4000..=0x7FFF => {
                match self.system_control.get_graphics_memory_map() {
                    GraphicsMemoryMap::FrameBuffer => {
                        let fb = self.system_control.banking_register.framebuffer() as usize;
                        self.framebuffers[fb].borrow_mut()[address as usize - 0x4000] = data;
                    }
                    GraphicsMemoryMap::VRAM => {
                        let vram_page = self.system_control.banking_register.vram_page() as usize;
                        let quadrant = self.blitter.vram_quadrant();
                        self.vram_banks[vram_page][address as usize - 0x4000 + quadrant*(128*128)] = data;
                        self.vram_quad_written[quadrant + vram_page * 4] = true;
                    }
                    GraphicsMemoryMap::BlitterRegisters => {
                        self.blitter.write_byte(address, data);
                        // println!("blitter reg write -> ${:04X}={:02X}", address, data);
                    }
                }
            }
            // Cartridge
            0x8000..=0xFFFF => {
                self.cartridge.write_byte(address - 0x8000, data);
            }
            _ => {
                warn!("Attempted to write read-only memory at: ${:02X}", address);
            }
        }
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            // system RAM
            0x0000..=0x1FFF => {
                return self.ram_banks[self.system_control.get_ram_bank()][address as usize];
            }

            // system control registers
            0x2000..=0x2009 => {
                return self.system_control.read_byte(address);
            }

            // versatile interface adapter (GPIO, timers)
            0x2800..=0x280F => {
                let register = (address & 0xF) as usize;
                return self.system_control.via_regs[register]
            }

            // audio RAM
            0x3000..=0x3FFF => unsafe {
                return ARAM[(address - 0x3000) as usize];
            }

            // VRAM/Framebuffer/Blitter
            0x4000..=0x7FFF => {
                match self.system_control.get_graphics_memory_map() {
                    GraphicsMemoryMap::FrameBuffer => {
                        let fb = self.system_control.banking_register.framebuffer() as usize;
                        return self.framebuffers[fb].borrow()[address as usize - 0x4000];
                    }
                    GraphicsMemoryMap::VRAM => {
                        let vram_page = self.system_control.banking_register.vram_page() as usize;
                        let quadrant = self.blitter.vram_quadrant();
                        return self.vram_banks[vram_page][address as usize - 0x4000 + quadrant*(128*128)];
                    }
                    GraphicsMemoryMap::BlitterRegisters => {
                        return self.blitter.read_byte(address);
                    }
                }
            }
            // Cartridge
            0x8000..=0xFFFF => {
                return self.cartridge.read_byte(address - 0x8000);
            }
            _ => {
                debug!("Attempted to inaccessible memory at: ${:02X}", address);
            }
        }

        0
    }

    pub fn peek_byte_decorated(&self, address: u16) -> ByteDecorator {
        match address {
            0x0000..=0x00FF => { ZeroPage(self.ram_banks[self.system_control.get_ram_bank()][address as usize]) },
            0x0100..=0x01FF => { CpuStack(self.ram_banks[self.system_control.get_ram_bank()][address as usize]) },
            0x0200..=0x1FFF => { SystemRam(self.ram_banks[self.system_control.get_ram_bank()][address as usize]) },
            0x2000..=0x2009 => { Unreadable(self.system_control.peek_byte(address)) },
            // 0x2800..=0x280F => { Via(self.system_control.via_regs[(address & 0xF) as usize]) },
            0x3000..=0x3FFF => unsafe { AudioRam(ARAM[(address - 0x3000) as usize]) },
            0x4000..=0x7FFF => {
                match self.system_control.get_graphics_memory_map() {
                    GraphicsMemoryMap::FrameBuffer => {
                        let fb = self.system_control.banking_register.framebuffer() as usize;
                        ByteDecorator::Framebuffer(self.framebuffers[fb].borrow()[address as usize - 0x4000])
                    }
                    GraphicsMemoryMap::VRAM => {
                        let vram_page = self.system_control.banking_register.vram_page() as usize;
                        let quadrant = self.blitter.vram_quadrant();
                        Vram(self.vram_banks[vram_page][address as usize - 0x4000 + quadrant*(128*128)])
                    }
                    GraphicsMemoryMap::BlitterRegisters => {
                        Unreadable(0)
                    }
                }
            },
            _ => Unreadable(0),
        }
    }

    pub fn vblank_nmi_enabled(&self) -> bool {
        self.system_control.dma_flags.dma_nmi()
    }
}

impl System for CpuBus {
    fn read(&mut self, _: &mut W65C02S, addr: u16) -> u8 {
        self.read_byte(addr)
    }

    fn write(&mut self, _: &mut W65C02S, addr: u16, data: u8) {
        self.write_byte(addr, data);
    }
}

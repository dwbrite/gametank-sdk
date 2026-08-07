use alloc::{boxed::Box, string::ToString, vec::Vec};
use log::warn;

use crate::{
    cartridges::Cartridge,
    gametank_bus::{DDRA, IORA},
};

/// Block lengths for the 35 blocks in the 2MB flash cartridge
const BLOCK_LENGTHS: [usize; 35] = [
    // last 31 blocks: 64KB each (0x10000 bytes)
    0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000,
    0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000,
    0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000,
    0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000, 0x10000,
    0x10000, 0x10000, 0x10000,

    0x08000,
    0x02000, 0x02000,
    0x04000,
];

/// Maps 16KB banks to their corresponding block indices and offsets within those blocks
#[derive(Debug, Copy, Clone)]
struct BlockMapping {
    block_index: u8,
    bank_offset: usize,
}

/// Generate the compile-time mapping of 128 banks to their underlying flash blocks
const fn generate_block_mappings() -> [BlockMapping; 128] {
    let mut mappings = [BlockMapping {
        block_index: 0,
        bank_offset: 0,
    }; 128];

    let mut block_index = 0;
    let mut offset_in_block = 0;
    let mut bank_index = 0;

    while bank_index < 128 && block_index < 35 {
        mappings[bank_index] = BlockMapping {
            block_index: block_index as u8,
            bank_offset: offset_in_block,
        };

        offset_in_block += 0x4000;
        bank_index += 1;

        // Move to next block if we've exhausted the current one
        if offset_in_block >= BLOCK_LENGTHS[block_index] {
            block_index += 1;
            offset_in_block = 0;
        }
    }

    mappings
}

const BLOCK_MAPPINGS: [BlockMapping; 128] = generate_block_mappings();

const BANK_SIZE: usize = 0x4000;  // 16KB per bank
const TOTAL_SIZE: usize = BANK_SIZE * 128;  // 2MB total (128 banks × 16KB)

/// 2MB Flash Cartridge implementation with bank switching and flash memory emulation
#[derive(Debug, Clone)]
pub struct Cartridge2M {
    data: Box<[u8; TOTAL_SIZE]>,
    pub bank_shifter: u8,
    pub bank_mask: u8,
    flash_state_machine: FlashStateMachine,
}

// VIA Port A bit masks
const CLK: u8 = 0b0000_0001;   // PA0: Clock signal
const DATA: u8 = 0b0000_0010;  // PA1: Data signal
const LATCH: u8 = 0b0000_0100; // PA2: Latch signal

// VIA array indices
const BEFORE: usize = 0;

/// Reverse the bits in a u8 value (hardware bug workaround)
const fn reverse_bits(mut value: u8) -> u8 {
    let mut result = 0u8;
    let mut i = 0;
    while i < 8 {
        result = (result << 1) | (value & 1);
        value >>= 1;
        i += 1;
    }
    result
}

/// Reverse only the lower 7 bits (for bank addressing, 0-127)
const fn reverse_bank_bits(mut value: u8) -> u8 {
    let mut result = 0u8;
    let mut i = 0;
    while i < 7 {
        result = (result << 1) | (value & 1);
        value >>= 1;
        i += 1;
    }
    result
}
const AFTER: usize = 1;

/// Represents a single flash memory write operation
#[derive(Debug, Copy, Clone)]
struct FlashInput {
    address: u16,
    data: u8,
}

impl FlashInput {
    const fn new(address: u16, data: u8) -> Self {
        Self { address, data }
    }
}

/// Circular buffer for tracking recent flash write commands
#[derive(Debug, Copy, Clone)]
struct FlashCmdBuffer {
    buffer: [FlashInput; 8],
}

impl FlashCmdBuffer {
    const fn new() -> Self {
        Self {
            buffer: [FlashInput { address: 0, data: 0 }; 8],
        }
    }

    fn add_input(&mut self, address: u16, data: u8) {
        // Shift existing inputs right to make room for new input at the front
        for i in (1..self.buffer.len()).rev() {
            self.buffer[i] = self.buffer[i - 1];
        }
        self.buffer[0] = FlashInput { address, data };
    }

    const fn get_inputs(&self) -> &[FlashInput] {
        &self.buffer
    }
}

/// Events detected from VIA Port A signal changes
#[derive(Debug)]
enum PaEvent {
    ClockRisingEdge,
    LatchRisingEdge,
    None,
}

/// Flash memory command types based on the SST39SF040 datasheet
#[derive(Debug, Copy, Clone, PartialEq)]
enum FlashCommand {
    ReadArray,
    Program(u16, u8),
    UnlockBypassEnter,
    UnlockBypassProgram(u16, u8),
    UnlockBypassReset,
    ChipErase,
    BlockErase(u16),
    Unknown,
}

impl FlashCommand {
    /// Parse a flash command from a sequence of write operations
    /// Commands follow the SST39SF040 datasheet specification
    fn from_sequence(sequence: &[FlashInput]) -> Self {
        match sequence {
            // Block erase: AAA:AA, 555:55, AAA:80, AAA:AA, 555:55, addr:30
            [FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: 0xAAA, data: 0x80 },
             FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: block_addr, data: 0x30 }, ..] => {
                FlashCommand::BlockErase(*block_addr)
            }

            // Byte program: AAA:AA, 555:55, AAA:A0, addr:data
            [FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: 0xAAA, data: 0xA0 },
             FlashInput { address, data }, ..] => {
                FlashCommand::Program(*address, *data)
            }

            // Unlock bypass mode entry: AAA:AA, 555:55, AAA:20
            [FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: 0xAAA, data: 0x20 }, ..] => {
                FlashCommand::UnlockBypassEnter
            }

            // Chip erase: AAA:AA, 555:55, AAA:80, AAA:AA, 555:55, AAA:10
            [FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: 0xAAA, data: 0x80 },
             FlashInput { address: 0xAAA, data: 0xAA },
             FlashInput { address: 0x555, data: 0x55 },
             FlashInput { address: 0xAAA, data: 0x10 }, ..] => {
                FlashCommand::ChipErase
            }

            // Unlock bypass program: addr:A0, addr:data (address must not be magic values)
            [FlashInput { address, data: 0xA0 },
             FlashInput { address: prog_addr, data: prog_data }, ..] 
                if *address != 0xAAA && *address != 0x555 => {
                    FlashCommand::UnlockBypassProgram(*prog_addr, *prog_data)
                }

            // Unlock bypass reset: addr:90, addr:00 (address must not be magic values)
            [FlashInput { address: addr1, data: 0x90 },
             FlashInput { address: addr2, data: 0x00 }, ..] 
                if *addr1 != 0xAAA && *addr1 != 0x555 => {
                    FlashCommand::UnlockBypassReset
                }

            _ => FlashCommand::Unknown,
        }
    }
}

/// Represents the various states of the flash memory state machine
#[derive(Debug, Clone)]
enum FlashState {
    Idle,
    UnlockBypass,
    CommandExecution(FlashCommand),
}

/// State machine for managing flash memory operations
#[derive(Debug, Clone)]
struct FlashStateMachine {
    state: FlashState,
    buffer: Vec<FlashInput>,
}

impl FlashStateMachine {
    fn new() -> Self {
        Self {
            state: FlashState::Idle,
            buffer: Vec::new(),
        }
    }

    /// Add a new write operation and handle state transitions
    fn add_input(&mut self, address: u16, data: u8) -> Option<FlashCommand> {
        // warn!("{:03X}:{:02X}", address, data);
        self.buffer.push(FlashInput { address, data });

        // Keep only the last 6 operations to limit buffer size
        if self.buffer.len() > 6 {
            self.buffer.drain(0..self.buffer.len() - 6);
        }

        match &mut self.state {
            FlashState::Idle => {
                if let Some(command) = self.detect_command() {
                    self.state = FlashState::CommandExecution(command.clone());
                    return Some(command);
                }
            }
            FlashState::UnlockBypass => {
                if self.buffer.len() >= 2 {
                    let last2 = &self.buffer[self.buffer.len() - 2..];
                    if last2[0].data == 0xA0 {
                        self.state = FlashState::CommandExecution(FlashCommand::UnlockBypassProgram(
                            last2[1].address,
                            last2[1].data,
                        ));
                        return Some(FlashCommand::UnlockBypassProgram(
                            last2[1].address,
                            last2[1].data,
                        ));
                    } else if last2[0].data == 0x90 && last2[1].data == 0x00 {
                        self.state = FlashState::Idle;
                        return Some(FlashCommand::UnlockBypassReset);
                    }
                }
            }
            FlashState::CommandExecution(_) => {
                self.state = FlashState::Idle;
            }
        }

        None
    }

    /// Detect a command sequence from the buffer
    fn detect_command(&self) -> Option<FlashCommand> {
        let len = self.buffer.len().min(6);
        if len < 2 {
            return None;
        }

        for window_size in (2..=len).rev() {
            let start = self.buffer.len() - window_size;
            let sequence = &self.buffer[start..];
            let command = FlashCommand::from_sequence(sequence);
            if command != FlashCommand::Unknown {
                return Some(command);
            }
        }

        None
    }

    /// Execute the current command
    fn execute_command(&mut self, cartridge_data: &mut [u8; TOTAL_SIZE], bank_mask: u8) {
        if let FlashState::CommandExecution(command) = &self.state {
            match command {
                FlashCommand::ReadArray => {}
                FlashCommand::Program(address, data) => {
                    warn!("Programming: address=0x{:04X}, data=0x{:02X}", address, data);
                    let bank = (bank_mask & 0x7F) as usize;
                    let offset = (address & 0x3FFF) as usize;
                    let range = Cartridge2M::bank_range(bank);
                    cartridge_data[range.start + offset] &= data;
                }
                FlashCommand::UnlockBypassProgram(address, data) => {
                    let bank = (bank_mask & 0x7F) as usize;
                    let offset = (address & 0x3FFF) as usize;
                    let range = Cartridge2M::bank_range(bank);
                    warn!(
                        "Bypass program: address=0x{:04X}, data=0x{:02X}, bank={}, offset=0x{:04X}",
                        address, data, bank, offset
                    );
                    cartridge_data[range.start + offset] &= data;
                }
                FlashCommand::ChipErase => {
                    warn!("Chip erase: all data set to 0xFF");
                    cartridge_data.fill(0xFF);
                }
                FlashCommand::BlockErase(block_addr) => {
                    let current_bank = (bank_mask & 0x7F) as usize;
                    let target_block = BLOCK_MAPPINGS[current_bank].block_index as usize;
                    let block_start: usize = BLOCK_LENGTHS[..target_block].iter().sum();
                    let block_end = block_start + BLOCK_LENGTHS[target_block];

                    let start_bank = block_start / BANK_SIZE;
                    let end_bank = (block_end + BANK_SIZE - 1) / BANK_SIZE;
                    let start_offset = block_start % BANK_SIZE;
                    let end_offset = block_end % BANK_SIZE;

                    let banks: Vec<_> = (start_bank..end_bank).collect();
                    let block_size = block_end - block_start;

                    warn!(
                        "Erasing {}k block on bank(s) {}{}",
                        block_size / 1024,
                        banks.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "),
                        if block_size == 8 * 1024 {
                            let half = if start_offset == 0 { "[1/2]" } else { "[2/2]" };
                            half
                        } else {
                            ""
                        }
                    );

                    for bank_index in banks {
                        if bank_index >= 128 {
                            continue;
                        }

                        let range = Cartridge2M::bank_range(bank_index);
                        if bank_index == start_bank && start_offset != 0 {
                            cartridge_data[range.start + start_offset..range.end].fill(0xFF);
                        } else if bank_index == end_bank - 1 && end_offset != 0 {
                            cartridge_data[range.start..range.start + end_offset].fill(0xFF);
                        } else {
                            cartridge_data[range].fill(0xFF);
                        }
                    }
                }
                FlashCommand::UnlockBypassEnter => {
                    warn!("Entering unlock bypass mode");
                    self.state = FlashState::UnlockBypass;
                }
                FlashCommand::UnlockBypassReset => {
                    warn!("Exiting unlock bypass mode");
                    self.state = FlashState::Idle;
                }
                FlashCommand::Unknown => {}
            }
        }

        self.state = FlashState::Idle;
        self.buffer.clear();
    }
}

impl Cartridge2M {
    /// Calculate the byte range for a given bank index
    fn bank_range(bank: usize) -> core::ops::Range<usize> {
        let start = bank * BANK_SIZE;
        let end = start + BANK_SIZE;
        start..end
    }

    /// Get a slice view of the specified bank
    fn bank_slice(&self, bank: usize) -> &[u8] {
        let range = Self::bank_range(bank);
        &self.data[range]
    }
}

impl Cartridge for Cartridge2M {
    fn from_slice(slice: &[u8]) -> Self {
        let mut data = Box::new([0u8; TOTAL_SIZE]);
        
        // Copy data bank by bank, reversing the bank numbers to match hardware bug
        let num_banks = (slice.len() + BANK_SIZE - 1) / BANK_SIZE;
        for logical_bank in 0..num_banks {
            let physical_bank = reverse_bank_bits(logical_bank as u8) as usize;
            
            let src_start = logical_bank * BANK_SIZE;
            let src_end = (src_start + BANK_SIZE).min(slice.len());
            let dst_range = Self::bank_range(physical_bank);
            
            let copy_len = src_end - src_start;
            data[dst_range.start..dst_range.start + copy_len]
                .copy_from_slice(&slice[src_start..src_end]);
        }
        
        Self {
            data,
            bank_shifter: 0,
            bank_mask: 0x7E,
            flash_state_machine: FlashStateMachine::new(),
        }
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x4000..=0x7FFF => {
                self.bank_slice(0x7F)[(address as usize) & 0x3FFF]
            }
            0x0000..=0x3FFF => {
                let bank = (self.bank_mask & 0x7F) as usize;
                self.bank_slice(bank)[(address as usize) & 0x3FFF]
            }
            _ => {
                panic!("how the hell did you get here?");
            }
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        let should_execute = self.flash_state_machine.add_input(address, data);
        if let Some(command) = should_execute {
            self.flash_state_machine
                .execute_command(&mut self.data, self.bank_mask);
        }
    }

    fn update_via(&mut self, via: &mut [[u8; 16]; 2]) {
        // Only process Port A if it's configured as input
        if via[AFTER][DDRA] == 1 {
            return;
        }

        let pa_before = via[BEFORE][IORA];
        let pa_after = via[AFTER][IORA];

        match pa_read(pa_before, pa_after) {
            PaEvent::ClockRisingEdge => {
                // Shift in the data bit on clock rising edge
                self.bank_shifter = (self.bank_shifter << 1) | pa_data_bit(pa_after);
            }
            PaEvent::LatchRisingEdge => {
                // Latch the accumulated bank value on latch rising edge
                // Reverse bits to correct hardware bug where bank pins were reversed
                self.bank_mask = reverse_bank_bits(self.bank_shifter);
            }
            PaEvent::None => {}
        }
    }
}

/// Detect rising edge events on Port A signals
#[inline(always)]
fn pa_read(pa_before: u8, pa_after: u8) -> PaEvent {
    let changed = pa_before ^ pa_after;
    
    if (pa_after & CLK) != 0 && (changed & CLK) != 0 {
        PaEvent::ClockRisingEdge
    } else if (pa_after & LATCH) != 0 && (changed & LATCH) != 0 {
        PaEvent::LatchRisingEdge
    } else {
        PaEvent::None
    }
}

/// Extract the data bit (PA1) from Port A
#[inline(always)]
fn pa_data_bit(pa: u8) -> u8 {
    (pa & DATA) >> 1
}

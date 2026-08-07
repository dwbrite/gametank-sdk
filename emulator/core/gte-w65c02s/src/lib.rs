#![allow(unused)]

//! This crate is a cycle-accurate simulator for the WDC W65C02S, the most
//! advanced direct descendent of that catalyst of the home computer
//! revolution, the 6502.
//!
//! This crate accurately simulates all bus signals of the W65C02S except RDY,
//! SOB, and BE, which can all be simulated by outside code. It is written in
//! such a way that the unused bus logic usually gets optimized out. Make sure
//! LTO is enabled in your `Cargo.toml` for a tremendous speedup:
//!
//! ```toml
//! [profile.release]
//! lto = true
//! ```
//!
//! This crate does not depend on any other libraries, including the standard
//! library.
//!
//! The W65C02S instruction set includes the original NMOS 6502 instructions,
//! the additional instructions supported by all CMOS 6502s, the "Rockwell bit
//! extensions" (`BBRx`/`BBSx`/`RMBx`/`SMBx`), and the `WAI` and `STP`
//! instructions.
//!
//! The accuracy of this simulation has been tested on the [`65test` test
//! suite](https://github.com/SolraBizna/65test), which contains over 4500
//! tests. In every single test, the simulator's bus traffic is *exactly* the
//! same as the real hardware—even down to the timing of IRQ and NMI responses.
//! This means that this simulator is suitable for prototyping and simulation
//! of real systems using the W65C02S processor, including systems based on the
//! W65C134S MCU.
//!
//! To use it, you will need an instance of [`W65C02S`](struct.W65C02S.html)
//! and an implementation of [`System`](trait.System.html). `W65C02S` simulates
//! the CPU; `System` must simulate the hardware attached to the bus (memory,
//! IO devices, et cetera).
//!
//! ```rust
//! use w65c02s::*;
//!
//! pub fn main() {
//!     let mut system = HelloWorldSystem::new();
//!     let mut cpu = W65C02S::new();
//!     while cpu.get_state() != State::Stopped { cpu.step(&mut system); }
//! }
//!
//! /// A simple system with 64K of RAM, along with an output-only "serial
//! /// port" mapped to $0000.
//! struct HelloWorldSystem {
//!     ram: [u8; 65536],
//! }
//!
//! impl HelloWorldSystem {
//!     pub fn new() -> HelloWorldSystem {
//!         // initialize RAM with all 0xFFs
//!         let mut ram = [0xFF; 65536];
//!         // initialize the message
//!         ram[0x0001..0x000F].copy_from_slice(b"Hello World!\n\0");
//!         // initialize the program
//!         ram[0x0200..0x020C].copy_from_slice(&[
//!             op::LDX_IMM, 0,     //   LDX #0
//!                                 // loop:
//!             op::LDA_ZPX, 1,     //   LDA $01, X
//!             op::BNE, 1,         //   BNE +
//!             op::STP,            //   STP
//!             op::STA_ZP, 0,      // + STA $00
//!             op::INC_X,          //   INX
//!             op::BRA, 0xF6,      //   BRA loop
//!         ]);
//!         // initialize the reset vector to point to $0200
//!         ram[0xFFFC..0xFFFE].copy_from_slice(&[0x00, 0x02]);
//!         HelloWorldSystem { ram }
//!     }
//! }
//!
//! impl System for HelloWorldSystem {
//!     fn read(&mut self, _cpu: &mut W65C02S, addr: u16) -> u8 {
//!         // all reads return RAM values directly
//!         self.ram[addr as usize]
//!     }
//!     fn write(&mut self, _cpu: &mut W65C02S, addr: u16, value: u8) {
//!         if addr == 0 {
//!             // writing address $0000 outputs on an ASCII-only "serial port"
//!             print!("{}", String::from_utf8_lossy(&[value]));
//!         }
//!         else {
//!             // all other writes write to RAM
//!             self.ram[addr as usize] = value
//!         }
//!     }
//! }
//! ```
//!
//! This simulator is based on the simulator in the original [ARS
//! Emulator](https://github.com/SolraBizna/ars-emu).
//!
//! # License
//!
//! w65c02s is distributed under the zlib license. The complete text is as
//! follows:
//!
//! > Copyright (c) 2019, Solra Bizna
//! > 
//! > This software is provided "as-is", without any express or implied
//! > warranty. In no event will the author be held liable for any damages
//! > arising from the use of this software.
//! > 
//! > Permission is granted to anyone to use this software for any purpose,
//! > including commercial applications, and to alter it and redistribute it
//! > freely, subject to the following restrictions:
//! > 
//! > 1. The origin of this software must not be misrepresented; you must not
//! > claim that you wrote the original software. If you use this software in a
//! > product, an acknowledgement in the product documentation would be
//! > appreciated but is not required.
//! > 2. Altered source versions must be plainly marked as such, and must not
//! > be misrepresented as being the original software.
//! > 3. This notice may not be removed or altered from any source
//! > distribution.

pub mod op;
mod addressing_modes;
mod instructions;

use addressing_modes::*;

pub static OPCODE_CYCLES: [i32; 256]= [ // was this worst case or...? who knows, who cares
    7, 6, 2, 0, 5, 3, 5, 5, 2, 3, 4, 0, 7, 5, 7, 7,
    3, 5, 5, 0, 5, 4, 6, 5, 1, 5, 4, 0, 7, 5, 7, 7,
    6, 6, 2, 0, 3, 3, 5, 5, 3, 3, 4, 0, 5, 5, 7, 7,
    2, 5, 5, 0, 4, 4, 6, 5, 1, 5, 4, 0, 5, 5, 7, 7,
    5, 6, 2, 0, 2, 3, 5, 5, 2, 3, 4, 0, 4, 5, 7, 7,
    3, 5, 5, 0, 3, 4, 6, 5, 1, 5, 2, 0, 9, 5, 7, 7,
    5, 6, 2, 0, 3, 3, 5, 5, 3, 3, 4, 0, 7, 5, 7, 7,
    2, 5, 5, 0, 4, 4, 6, 5, 1, 5, 2, 0, 7, 5, 7, 7,
    3, 5, 2, 0, 3, 2, 3, 5, 4, 3, 1, 0, 5, 5, 5, 6,
    3, 5, 4, 0, 4, 3, 4, 5, 1, 5, 1, 0, 5, 5, 6, 6,
    3, 6, 3, 0, 3, 3, 3, 5, 1, 2, 1, 0, 5, 5, 5, 6,
    2, 5, 5, 0, 4, 4, 4, 5, 1, 5, 1, 0, 5, 5, 5, 6,
    3, 6, 2, 0, 3, 3, 5, 5, 4, 3, 4, 0, 5, 5, 7, 6,
    3, 5, 5, 0, 3, 4, 6, 5, 1, 5, 2, 1, 4, 5, 8, 6,
    3, 6, 2, 0, 3, 3, 5, 5, 4, 3, 1, 0, 5, 5, 7, 6,
    2, 5, 5, 0, 3, 4, 6, 5, 1, 6, 3, 0, 4, 6, 8, 6,
];

/// Implements a system connected to a W65C02S's bus. Only `read` and `write`
/// need be implemented for a simple system, but other systems may be more
/// complicated; for instance, many 65C02-based microcontrollers have advanced
/// interrupt vectoring logic that would require implementing `read_vector`.
pub trait System {
    /// Read an instruction opcode from the given address. VPB, MLB, and SYNC
    /// are all HIGH.
    fn read_opcode(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Read a byte of data from the given address. SYNC is LOW and VPB and MLB
    /// are HIGH.
    fn read(&mut self, cpu: &mut W65C02S, addr: u16) -> u8;
    /// Read data from the given address as part of a Read-Modify-Write
    /// instruction. SYNC and MLB are LOW, VPB is HIGH.
    fn read_locked(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Second data read from the given address as part of a Read-Modify-Write
    /// instruction. This data is ignored; this is an "idle cycle". SYNC and
    /// MLB are LOW, VPB is HIGH. Indistinguishable from a locked data read on
    /// real hardware, but the distinction may be useful for simulation.
    fn read_locked_spurious(&mut self, cpu: &mut W65C02S, addr: u16) { self.read_locked(cpu, addr); }
    /// Read part of an interrupt vector from the given address. VPB is LOW,
    /// and SYNC and MLB are HIGH.
    fn read_vector(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Write a byte of data to the given address. SYNC is LOW and VPB and MLB
    /// are HIGH.
    fn write(&mut self, cpu: &mut W65C02S, addr: u16, data: u8);
    /// Push a byte of data onto the stack at the given address. SYNC is LOW
    /// and VPB and MLB are HIGH. Indistinguishable from a normal data write
    /// on real hardware, but the distinction may be useful for simulation.
    fn write_stack(&mut self, cpu: &mut W65C02S, addr: u16, data: u8) { self.write(cpu, addr, data) }
    /// Write a byte of data to the given address as the conclusion of a Read-
    /// Modify-Write instruction. SYNC and MLB are LOW, VPB is HIGH.
    fn write_locked(&mut self, cpu: &mut W65C02S, addr: u16, data: u8) { self.write(cpu, addr, data) }
    /// Read an instruction opcode whose execution will be preempted by an
    /// interrupt or a reset, or which follows a WAI or STP instruction that
    /// has not yet been broken out of. VPB, MLB, and SYNC are all HIGH.
    /// Indistinguishable from a normal opcode fetch on real hardware, but
    /// the distinction may be useful for simulation.
    fn read_opcode_spurious(&mut self, cpu: &mut W65C02S, addr: u16) { self.read_opcode(cpu, addr); }
    /// Read an instruction operand from the given address. SYNC is LOW and VPB
    /// and MLB are HIGH. Indistinguishable from an ordinary data read on real
    /// hardware, but the distinction may be useful for simulation.
    fn read_operand(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Read an instruction operand from the given address, except that the
    /// instruction had an implied operand or was preempted by a reset. SYNC
    /// is LOW and VPB and MLB are HIGH. Indistinguishable from an ordinary
    /// data read on real hardware, but the distinction may be useful for
    /// simulation.
    fn read_operand_spurious(&mut self, cpu: &mut W65C02S, addr: u16) { self.read(cpu, addr); }
    /// Read part of a pointer from the given address. SYNC is LOW and VPB and
    /// MLB are HIGH. Indistinguishable from an ordinary data read on real
    /// hardware, but the distinction may be useful for simulation.
    fn read_pointer(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Pop a value from the stack at the given address. SYNC is LOW and VPB
    /// and MLB are HIGH. Indistinguishable from an ordinary data read on real
    /// hardware, but the distinction may be useful for simulation.
    fn read_stack(&mut self, cpu: &mut W65C02S, addr: u16) -> u8 { self.read(cpu, addr) }
    /// Spurious stack "read" that occurs during reset. SYNC is LOW and VPB
    /// and MLB are HIGH. Indistinguishable from an ordinary data read on real
    /// hardware, but the distinction may be useful for simulation.
    fn read_stack_spurious(&mut self, cpu: &mut W65C02S, addr: u16) { self.read_stack(cpu, addr); }
    /// Read a byte of data from the given address during an "internal
    /// operation" cycle. SYNC is LOW and VPB and MLB are HIGH.
    /// Indistinguishable from an ordinary data read on real hardware, but the
    /// distinction may be useful for simulation.
    fn read_spurious(&mut self, cpu: &mut W65C02S, addr: u16) { self.read(cpu, addr); }
}

/// Status register flag corresponding to the **C**arry bit.
pub const P_C: u8 = 0x01;
/// Status register flag corresponding to the **Z**ero bit.
pub const P_Z: u8 = 0x02;
/// Status register flag corresponding to the **I**nterrupt mask bit.
pub const P_I: u8 = 0x04;
/// Status register flag corresponding to the **D**ecimal mode bit.
pub const P_D: u8 = 0x08;
/// Status register flag corresponding to the **B**reak bit.
pub const P_B: u8 = 0x10;
/// Status register flag that is hardwired to 1, in spite of what the datasheet
/// says.
pub const P_1: u8 = 0x20;
/// Status register flag corresponding to the o**V**erflow bit.
pub const P_V: u8 = 0x40;
/// Status register flag corresponding to the **N**egative bit.
pub const P_N: u8 = 0x80;

/// Address of the IRQ/BRK interrupt vector.
pub const IRQ_VECTOR: u16 = 0xfffe;
/// Address of the Reset interrupt vector.
pub const RESET_VECTOR: u16 = 0xfffc;
/// Address of the NMI interrupt vector.
pub const NMI_VECTOR: u16 = 0xfffa;

/// The CPU is in one of the given states between `step`s.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum State {
    /// The CPU has just been reset. It will execute the reset sequence at the
    /// next step. RDY is HIGH.
    HasBeenReset,
    /// The CPU is in its normal operating state. It will fetch an instruction
    /// at the next step, and possibly handle an interrupt. RDY is HIGH.
    Running,
    /// The CPU has executed a `WAI` instruction and has not yet detected an
    /// interrupt. RDY is LOW.
    AwaitingInterrupt,
    /// The CPU has executed a `STP` instruction. Only a reset will return it
    /// to an executing state. RDY is LOW.
    Stopped,
}

/// An instance of a W65C02S, encapsulating the entire runtime state of the
/// processor itself. Not very useful without a `System` to go with it.
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub struct W65C02S {
    state: State, pc: u16,
    a: u8, x: u8, y: u8, s: u8, p: u8,
    irq: bool, irq_pending: bool,
    nmi: bool, nmi_edge: bool, nmi_pending: bool,
}

impl W65C02S {
    /// Creates a new `W65C02S` instance, initialized in a newly-reset state.
    /// (Unlike a real W65C02S, most registers will be in the all-ones state at
    /// this point.)
    pub fn new() -> W65C02S {
        W65C02S {
            state: State::HasBeenReset,
            pc: 0xFFFF,
            a: 0xFF,
            x: 0xFF,
            y: 0xFF,
            s: 0xFF,
            p: P_1|P_I,
            irq: false, irq_pending: false,
            nmi: false, nmi_edge: false, nmi_pending: false,
        }
    }
    /// Resets the CPU. Execution will flounder for a few cycles, then fetch
    /// the reset vector and "start over".
    #[inline(always)]
    pub fn reset(&mut self) {
        self.state = State::HasBeenReset;
        self.s = 0;
        self.p = P_1|P_I;
    }
    /// Get the current value of the **P**rogram **C**ounter, i.e. the next
    /// instruction that will (probably) be executed.
    #[inline(always)]
    pub fn get_pc(&self) -> u16 { self.pc }
    /// Internal function. Get the current value of the **P**rogram
    /// **C**ounter. Increment the underlying value by one *after* reading it.
    #[inline(always)]
    fn read_pc_postincrement(&mut self) -> u16 {
        let ret = self.pc;
        self.pc = self.pc.wrapping_add(1);
        ret
    }
    /// Overwrite the current value of the **P**rogram **C**ounter.
    #[inline(always)]
    pub fn set_pc(&mut self, pc: u16) { self.pc = pc }
    /// Get the current value of the **A**ccumulator.
    #[inline(always)]
    pub fn get_a(&self) -> u8 { self.a }
    /// Overwrite the current value of the **A**ccumulator.
    #[inline(always)]
    pub fn set_a(&mut self, a: u8) { self.a = a }
    /// Get the current value of index register **X**.
    #[inline(always)]
    pub fn get_x(&self) -> u8 { self.x }
    /// Overwrite the current value of index register **X**.
    #[inline(always)]
    pub fn set_x(&mut self, x: u8) { self.x = x }
    /// Get the current value of index register **Y**.
    #[inline(always)]
    pub fn get_y(&self) -> u8 { self.y }
    /// Overwrite the current value of index register **Y**.
    #[inline(always)]
    pub fn set_y(&mut self, y: u8) { self.y = y }
    /// Get the current value of the **S**tack pointer. The next byte pushed
    /// onto the stack will go into address `$01xx` where `xx` is this value.
    #[inline(always)]
    pub fn get_s(&self) -> u8 { self.s }
    /// Overwrite the current value of the **S**tack pointer.
    #[inline(always)]
    pub fn set_s(&mut self, s: u8) { self.s = s }
    /// Get the current value of the **P**rocessor status register. Use the
    /// `P_*` constants to interpret the value.
    #[inline(always)]
    pub fn get_p(&self) -> u8 { self.p }
    /// Overwrite the current value of the **P**rocessor status register.
    /// Can't be used to change the hardwired 1 bit. You can use this to
    /// implement SOB logic; when the SOB pin should transition to active, do
    /// something like:
    ///
    /// ```rust
    /// # use w65c02s::*;
    /// # let mut cpu = W65C02S::new();
    /// cpu.set_p(cpu.get_p() | P_V);
    /// ```
    #[inline(always)]
    pub fn set_p(&mut self, p: u8) { self.p = p | P_1 }
    /// Get the current operating state of the CPU. May return a stale value if
    /// called during a `step`.
    #[inline(always)]
    pub fn get_state(&self) -> State { self.state }
    /// Push a value onto the stack using the given `System`.
    #[inline(always)]
    pub fn push<S: System>(&mut self, system: &mut S, value: u8) {
        system.write_stack(self, 0x100 | self.s as u16, value);
        self.s = self.s.wrapping_sub(1);
    }
    /// Spurious push during reset.
    #[inline(always)]
    pub fn spurious_push<S: System>(&mut self, system: &mut S) {
        // system.read_stack_spurious(self, 0x100 | self.s as u16);
        self.s = self.s.wrapping_sub(1);
    }
    /// Pop a value from the stack using the given `System`.
    #[inline(always)]
    pub fn pop<S: System>(&mut self, system: &mut S) -> u8 {
        self.s = self.s.wrapping_add(1);
        system.read_stack(self, 0x100 | self.s as u16)
    }
    /// Spuriously read a value from the next stack slot, like happens during
    /// a JSR or RTS or most pulls.
    #[inline(always)]
    pub fn spurious_stack_read<S: System>(&mut self, system: &mut S) {
        // system.read_spurious(self, 0x100 | (self.s as u16));
    }
    /// Change the input on the `IRQB` pin. `false` means no interrupt pending.
    /// `true` means some interrupt is pending. Note that `IRQB` is an active-
    /// low pin and that the value you pass to this function is the *logical*
    /// value and not the *electrical* one.
    #[inline(always)]
    pub fn set_irq(&mut self, irq: bool) { self.irq = irq }
    /// Change the input on the NMIB pin. `false` means no NMI pending. A
    /// transition from `false` to `true` triggers an NMI at the next `step`.
    /// Note that `NMIB` is an active-low pin and that the value you pass to
    /// this function is the *logical* value and not the *electrical* one.
    ///
    /// This crate does not accurately simulate extremely short NMI pulses, or
    /// extremely rapid ones. If these conditions arise on real hardware, chaos
    /// will ensue anyway.
    #[inline(always)]
    pub fn set_nmi(&mut self, nmi: bool) {
        self.nmi_edge = self.nmi_edge || (!self.nmi_edge && nmi);
        self.nmi = nmi;
    }
    /// Internal function. Updates the IRQ and NMI edge flags.
    #[inline(always)]
    fn check_irq_edge(&mut self) {
        self.irq_pending = self.irq && (self.p & P_I) == 0;
        self.nmi_pending = self.nmi_edge;
    }
    /// Set N and Z flags according to the argument value.
    #[inline(always)]
    fn nz_p(&mut self, v: u8) {
        self.p = (self.p & 0x7F) | (v & 0x80);
        if v == 0 { self.p |= P_Z; }
        else { self.p &= !P_Z; }
    }
    /// Set N and Z flags according to the argument value, and the C flag
    /// according to the argument.
    #[inline(always)]
    fn cnz_p(&mut self, c: bool, v: u8) {
        self.p = (self.p & 0x7F) | (v & 0x80);
        if v == 0 { self.p |= P_Z; }
        else { self.p &= !P_Z; }
        if c { self.p |= P_C; }
        else { self.p &= !P_C; }
    }
    /// Step the processor once. This means executing an interrupt sequence,
    /// fetching an instruction, or doing a spurious read, depending on the
    /// current state of the processor. Returns the new state.
    ///
    /// Always executes at least one bus cycle. May execute more.
    pub fn step<S: System>(&mut self, system: &mut S) -> i32 {
        match self.state {
            State::Stopped => /*system.read_operand_spurious(self, self.pc),*/return 1,
            State::AwaitingInterrupt => {
                if self.irq || self.nmi_edge {
                    self.state = State::Running;
                    // system.read_operand_spurious(self, self.pc);
                }
                self.check_irq_edge();
                // system.read_operand_spurious(self, self.pc);

                return 2;
            },
            State::HasBeenReset => {
                // first, we spuriously read an opcode
                // system.read_opcode_spurious(self, self.pc);
                // second, we read ... the same byte, but with SYNC low
                // system.read_operand_spurious(self, self.pc);
                // three spurious pushes...
                self.spurious_push(system);
                self.spurious_push(system);
                self.spurious_push(system);
                // clear the D flag, set the I flag
                self.p &= !P_D;
                self.p |= P_I;
                // read the reset vector, non-spuriously
                self.pc = (self.pc & 0xFF00) | (system.read_vector(self, RESET_VECTOR) as u16);
                self.pc = (self.pc & 0x00FF) | (system.read_vector(self, RESET_VECTOR+1) as u16) << 8;
                // we are ready to be actually running!
                self.state = State::Running;

                return 7
            },
            State::Running => {
                if self.nmi_pending {
                    self.nmi_pending = false;
                    self.nmi_edge = false;
                    let opcode_addr = self.get_pc();
                    // system.read_opcode_spurious(self, opcode_addr);
                    // system.read_spurious(self, opcode_addr);
                    self.push(system, (opcode_addr >> 8) as u8);
                    self.push(system, opcode_addr as u8);
                    self.push(system, self.p & !P_B);
                    self.p &= !P_D;
                    self.p |= P_I;
                    self.pc = (self.pc & 0xFF00) | (system.read_vector(self, NMI_VECTOR) as u16);
                    self.pc = (self.pc & 0x00FF) | (system.read_vector(self, NMI_VECTOR+1) as u16) << 8;
                    return 7
                }
                else if self.irq_pending {
                    self.irq_pending = false;
                    let opcode_addr = self.get_pc();
                    // system.read_opcode_spurious(self, opcode_addr);
                    // system.read_spurious(self, opcode_addr);
                    self.push(system, (opcode_addr >> 8) as u8);
                    self.push(system, opcode_addr as u8);
                    self.push(system, self.p);
                    self.p &= !P_D;
                    self.p |= P_I;
                    self.pc = (self.pc & 0xFF00) | (system.read_vector(self, IRQ_VECTOR) as u16);
                    self.pc = (self.pc & 0x00FF) | (system.read_vector(self, IRQ_VECTOR+1) as u16) << 8;
                    return 7
                }
                else {
                    // oh boy, we're running! oh boy oh boy!
                    let opcode_addr = self.read_pc_postincrement();
                    let opcode = system.read_opcode(self, opcode_addr);
                    // oh boy OH JEEZ
                    match opcode {
                        0x00 => self.brk(system),
                        0x01 => self.ora::<_, ZeroPageXIndirect, S>(system),
                        0x02 => self.nop::<_, Immediate, S>(system),
                        0x03 => self.nop::<_, FastImplied, S>(system),
                        0x04 => self.tsb::<_, ZeroPage, S>(system),
                        0x05 => self.ora::<_, ZeroPage, S>(system),
                        0x06 => self.asl::<_, ZeroPage, S>(system),
                        0x07 => self.rmb::<_, ZeroPage, S>(system, !0x01),
                        0x08 => self.php(system),
                        0x09 => self.ora::<_, Immediate, S>(system),
                        0x0A => self.asl::<_, ImpliedA, S>(system),
                        0x0B => self.nop::<_, FastImplied, S>(system),
                        0x0C => self.tsb::<_, Absolute, S>(system),
                        0x0D => self.ora::<_, Absolute, S>(system),
                        0x0E => self.asl::<_, Absolute, S>(system),
                        0x0F => self.bbr::<_, RelativeBitBranch, S>(system, 0x01),
                        0x10 => self.branch::<_, Relative, S>(system, self.p & P_N == 0),
                        0x11 => self.ora::<_, ZeroPageIndirectY, S>(system),
                        0x12 => self.ora::<_, ZeroPageIndirect, S>(system),
                        0x13 => self.nop::<_, FastImplied, S>(system),
                        0x14 => self.trb::<_, ZeroPage, S>(system),
                        0x15 => self.ora::<_, ZeroPageX, S>(system),
                        0x16 => self.asl::<_, ZeroPageX, S>(system),
                        0x17 => self.rmb::<_, ZeroPage, S>(system, !0x02),
                        0x18 => self.clc(system),
                        0x19 => self.ora::<_, AbsoluteY, S>(system),
                        0x1A => self.inc::<_, ImpliedA, S>(system),
                        0x1B => self.nop::<_, FastImplied, S>(system),
                        0x1C => self.trb::<_, Absolute, S>(system),
                        0x1D => self.ora::<_, AbsoluteX, S>(system),
                        0x1E => self.asl::<_, AbsoluteX, S>(system),
                        0x1F => self.bbr::<_, RelativeBitBranch, S>(system, 0x02),
                        0x20 => self.jsr(system),
                        0x21 => self.and::<_, ZeroPageXIndirect, S>(system),
                        0x22 => self.nop::<_, Immediate, S>(system),
                        0x23 => self.nop::<_, FastImplied, S>(system),
                        0x24 => self.bit::<_, ZeroPage, S>(system),
                        0x25 => self.and::<_, ZeroPage, S>(system),
                        0x26 => self.rol::<_, ZeroPage, S>(system),
                        0x27 => self.rmb::<_, ZeroPage, S>(system, !0x04),
                        0x28 => self.plp(system),
                        0x29 => self.and::<_, Immediate, S>(system),
                        0x2A => self.rol::<_, ImpliedA, S>(system),
                        0x2B => self.nop::<_, FastImplied, S>(system),
                        0x2C => self.bit::<_, Absolute, S>(system),
                        0x2D => self.and::<_, Absolute, S>(system),
                        0x2E => self.rol::<_, Absolute, S>(system),
                        0x2F => self.bbr::<_, RelativeBitBranch, S>(system, 0x04),
                        0x30 => self.branch::<_, Relative, S>(system, self.p & P_N == P_N),
                        0x31 => self.and::<_, ZeroPageIndirectY, S>(system),
                        0x32 => self.and::<_, ZeroPageIndirect, S>(system),
                        0x33 => self.nop::<_, FastImplied, S>(system),
                        0x34 => self.bit::<_, ZeroPageX, S>(system),
                        0x35 => self.and::<_, ZeroPageX, S>(system),
                        0x36 => self.rol::<_, ZeroPageX, S>(system),
                        0x37 => self.rmb::<_, ZeroPage, S>(system, !0x08),
                        0x38 => self.sec(system),
                        0x39 => self.and::<_, AbsoluteY, S>(system),
                        0x3A => self.dec::<_, ImpliedA, S>(system),
                        0x3B => self.nop::<_, FastImplied, S>(system),
                        0x3C => self.bit::<_, AbsoluteX, S>(system),
                        0x3D => self.and::<_, AbsoluteX, S>(system),
                        0x3E => self.rol::<_, AbsoluteX, S>(system),
                        0x3F => self.bbr::<_, RelativeBitBranch, S>(system, 0x08),
                        0x40 => self.rti(system),
                        0x41 => self.eor::<_, ZeroPageXIndirect, S>(system),
                        0x42 => self.nop::<_, Immediate, S>(system),
                        0x43 => self.nop::<_, FastImplied, S>(system),
                        0x44 => self.nop::<_, ZeroPage, S>(system),
                        0x45 => self.eor::<_, ZeroPage, S>(system),
                        0x46 => self.lsr::<_, ZeroPage, S>(system),
                        0x47 => self.rmb::<_, ZeroPage, S>(system, !0x10),
                        0x48 => self.pha(system),
                        0x49 => self.eor::<_, Immediate, S>(system),
                        0x4A => self.lsr::<_, ImpliedA, S>(system),
                        0x4B => self.nop::<_, FastImplied, S>(system),
                        0x4C => self.jmp::<_, Absolute, S>(system),
                        0x4D => self.eor::<_, Absolute, S>(system),
                        0x4E => self.lsr::<_, Absolute, S>(system),
                        0x4F => self.bbr::<_, RelativeBitBranch, S>(system, 0x10),
                        0x50 => self.branch::<_, Relative, S>(system, self.p & P_V == 0),
                        0x51 => self.eor::<_, ZeroPageIndirectY, S>(system),
                        0x52 => self.eor::<_, ZeroPageIndirect, S>(system),
                        0x53 => self.nop::<_, FastImplied, S>(system),
                        0x54 => self.nop::<_, ZeroPageX, S>(system),
                        0x55 => self.eor::<_, ZeroPageX, S>(system),
                        0x56 => self.lsr::<_, ZeroPageX, S>(system),
                        0x57 => self.rmb::<_, ZeroPage, S>(system, !0x20),
                        0x58 => self.cli(system),
                        0x59 => self.eor::<_, AbsoluteY, S>(system),
                        0x5A => self.phy(system),
                        0x5B => self.nop::<_, FastImplied, S>(system),
                        0x5C => self.nop_5c::<_, Absolute, S>(system),
                        0x5D => self.eor::<_, AbsoluteX, S>(system),
                        0x5E => self.lsr::<_, AbsoluteX, S>(system),
                        0x5F => self.bbr::<_, RelativeBitBranch, S>(system, 0x20),
                        0x60 => self.rts(system),
                        0x61 => self.adc::<_, ZeroPageXIndirect, S>(system),
                        0x62 => self.nop::<_, Immediate, S>(system),
                        0x63 => self.nop::<_, FastImplied, S>(system),
                        0x64 => self.stz::<_, ZeroPage, S>(system),
                        0x65 => self.adc::<_, ZeroPage, S>(system),
                        0x66 => self.ror::<_, ZeroPage, S>(system),
                        0x67 => self.rmb::<_, ZeroPage, S>(system, !0x40),
                        0x68 => self.pla(system),
                        0x69 => self.adc::<_, Immediate, S>(system),
                        0x6A => self.ror::<_, ImpliedA, S>(system),
                        0x6B => self.nop::<_, FastImplied, S>(system),
                        0x6C => self.jmp::<_, AbsoluteIndirect, S>(system),
                        0x6D => self.adc::<_, Absolute, S>(system),
                        0x6E => self.ror::<_, Absolute, S>(system),
                        0x6F => self.bbr::<_, RelativeBitBranch, S>(system, 0x40),
                        0x70 => self.branch::<_, Relative, S>(system, self.p & P_V == P_V),
                        0x71 => self.adc::<_, ZeroPageIndirectY, S>(system),
                        0x72 => self.adc::<_, ZeroPageIndirect, S>(system),
                        0x73 => self.nop::<_, FastImplied, S>(system),
                        0x74 => self.stz::<_, ZeroPageX, S>(system),
                        0x75 => self.adc::<_, ZeroPageX, S>(system),
                        0x76 => self.ror::<_, ZeroPageX, S>(system),
                        0x77 => self.rmb::<_, ZeroPage, S>(system, !0x80),
                        0x78 => self.sei(system),
                        0x79 => self.adc::<_, AbsoluteY, S>(system),
                        0x7A => self.ply(system),
                        0x7B => self.nop::<_, FastImplied, S>(system),
                        0x7C => self.jmp::<_, AbsoluteXIndirect, S>(system),
                        0x7D => self.adc::<_, AbsoluteX, S>(system),
                        0x7E => self.ror::<_, AbsoluteX, S>(system),
                        0x7F => self.bbr::<_, RelativeBitBranch, S>(system, 0x80),
                        0x80 => self.branch::<_, Relative, S>(system, true),
                        0x81 => self.sta::<_, ZeroPageXIndirect, S>(system),
                        0x82 => self.nop::<_, Immediate, S>(system),
                        0x83 => self.nop::<_, FastImplied, S>(system),
                        0x84 => self.sty::<_, ZeroPage, S>(system),
                        0x85 => self.sta::<_, ZeroPage, S>(system),
                        0x86 => self.stx::<_, ZeroPage, S>(system),
                        0x87 => self.smb::<_, ZeroPage, S>(system, 0x01),
                        0x88 => self.dec::<_, ImpliedY, S>(system),
                        0x89 => self.bit_i::<_, Immediate, S>(system),
                        0x8A => self.txa(system),
                        0x8B => self.nop::<_, FastImplied, S>(system),
                        0x8C => self.sty::<_, Absolute, S>(system),
                        0x8D => self.sta::<_, Absolute, S>(system),
                        0x8E => self.stx::<_, Absolute, S>(system),
                        0x8F => self.bbs::<_, RelativeBitBranch, S>(system, 0x01),
                        0x90 => self.branch::<_, Relative, S>(system, self.p & P_C == 0),
                        0x91 => self.sta::<_, ZeroPageIndirectYSlower, S>(system),
                        0x92 => self.sta::<_, ZeroPageIndirect, S>(system),
                        0x93 => self.nop::<_, FastImplied, S>(system),
                        0x94 => self.sty::<_, ZeroPageX, S>(system),
                        0x95 => self.sta::<_, ZeroPageX, S>(system),
                        0x96 => self.stx::<_, ZeroPageY, S>(system),
                        0x97 => self.smb::<_, ZeroPage, S>(system, 0x02),
                        0x98 => self.tya(system),
                        0x99 => self.sta::<_, AbsoluteYSlower, S>(system),
                        0x9A => self.txs(system),
                        0x9B => self.nop::<_, FastImplied, S>(system),
                        0x9C => self.stz::<_, Absolute, S>(system),
                        0x9D => self.sta::<_, AbsoluteXSlower, S>(system),
                        0x9E => self.stz::<_, AbsoluteXSlower, S>(system),
                        0x9F => self.bbs::<_, RelativeBitBranch, S>(system, 0x02),
                        0xA0 => self.ldy::<_, Immediate, S>(system),
                        0xA1 => self.lda::<_, ZeroPageXIndirect, S>(system),
                        0xA2 => self.ldx::<_, Immediate, S>(system),
                        0xA3 => self.nop::<_, FastImplied, S>(system),
                        0xA4 => self.ldy::<_, ZeroPage, S>(system),
                        0xA5 => self.lda::<_, ZeroPage, S>(system),
                        0xA6 => self.ldx::<_, ZeroPage, S>(system),
                        0xA7 => self.smb::<_, ZeroPage, S>(system, 0x04),
                        0xA8 => self.tay(system),
                        0xA9 => self.lda::<_, Immediate, S>(system),
                        0xAA => self.tax(system),
                        0xAB => self.nop::<_, FastImplied, S>(system),
                        0xAC => self.ldy::<_, Absolute, S>(system),
                        0xAD => self.lda::<_, Absolute, S>(system),
                        0xAE => self.ldx::<_, Absolute, S>(system),
                        0xAF => self.bbs::<_, RelativeBitBranch, S>(system, 0x04),
                        0xB0 => self.branch::<_, Relative, S>(system, self.p & P_C == P_C),
                        0xB1 => self.lda::<_, ZeroPageIndirectY, S>(system),
                        0xB2 => self.lda::<_, ZeroPageIndirect, S>(system),
                        0xB3 => self.nop::<_, FastImplied, S>(system),
                        0xB4 => self.ldy::<_, ZeroPageX, S>(system),
                        0xB5 => self.lda::<_, ZeroPageX, S>(system),
                        0xB6 => self.ldx::<_, ZeroPageY, S>(system),
                        0xB7 => self.smb::<_, ZeroPage, S>(system, 0x08),
                        0xB8 => self.clv(system),
                        0xB9 => self.lda::<_, AbsoluteY, S>(system),
                        0xBA => self.tsx(system),
                        0xBB => self.nop::<_, FastImplied, S>(system),
                        0xBC => self.ldy::<_, AbsoluteX, S>(system),
                        0xBD => self.lda::<_, AbsoluteX, S>(system),
                        0xBE => self.ldx::<_, AbsoluteY, S>(system),
                        0xBF => self.bbs::<_, RelativeBitBranch, S>(system, 0x08),
                        0xC0 => self.cpy::<_, Immediate, S>(system),
                        0xC1 => self.cmp::<_, ZeroPageXIndirect, S>(system),
                        0xC2 => self.nop::<_, Immediate, S>(system),
                        0xC3 => self.nop::<_, FastImplied, S>(system),
                        0xC4 => self.cpy::<_, ZeroPage, S>(system),
                        0xC5 => self.cmp::<_, ZeroPage, S>(system),
                        0xC6 => self.dec::<_, ZeroPage, S>(system),
                        0xC7 => self.smb::<_, ZeroPage, S>(system, 0x10),
                        0xC8 => self.inc::<_, ImpliedY, S>(system),
                        0xC9 => self.cmp::<_, Immediate, S>(system),
                        0xCA => self.dec::<_, ImpliedX, S>(system),
                        0xCB => self.wai(system),
                        0xCC => self.cpy::<_, Absolute, S>(system),
                        0xCD => self.cmp::<_, Absolute, S>(system),
                        0xCE => self.dec::<_, Absolute, S>(system),
                        0xCF => self.bbs::<_, RelativeBitBranch, S>(system, 0x10),
                        0xD0 => self.branch::<_, Relative, S>(system, self.p & P_Z == 0),
                        0xD1 => self.cmp::<_, ZeroPageIndirectY, S>(system),
                        0xD2 => self.cmp::<_, ZeroPageIndirect, S>(system),
                        0xD3 => self.nop::<_, FastImplied, S>(system),
                        0xD4 => self.nop::<_, ZeroPageX, S>(system),
                        0xD5 => self.cmp::<_, ZeroPageX, S>(system),
                        0xD6 => self.dec::<_, ZeroPageX, S>(system),
                        0xD7 => self.smb::<_, ZeroPage, S>(system, 0x20),
                        0xD8 => self.cld(system),
                        0xD9 => self.cmp::<_, AbsoluteY, S>(system),
                        0xDA => self.phx(system),
                        0xDB => self.stp(system),
                        0xDC => self.nop::<_, Absolute, S>(system),
                        0xDD => self.cmp::<_, AbsoluteX, S>(system),
                        0xDE => self.dec::<_, AbsoluteXSlower, S>(system),
                        0xDF => self.bbs::<_, RelativeBitBranch, S>(system, 0x20),
                        0xE0 => self.cpx::<_, Immediate, S>(system),
                        0xE1 => self.sbc::<_, ZeroPageXIndirect, S>(system),
                        0xE2 => self.nop::<_, Immediate, S>(system),
                        0xE3 => self.nop::<_, FastImplied, S>(system),
                        0xE4 => self.cpx::<_, ZeroPage, S>(system),
                        0xE5 => self.sbc::<_, ZeroPage, S>(system),
                        0xE6 => self.inc::<_, ZeroPage, S>(system),
                        0xE7 => self.smb::<_, ZeroPage, S>(system, 0x40),
                        0xE8 => self.inc::<_, ImpliedX, S>(system),
                        0xE9 => self.sbc::<_, Immediate, S>(system),
                        0xEA => self.nop::<_, Implied, S>(system),
                        0xEB => self.nop::<_, FastImplied, S>(system),
                        0xEC => self.cpx::<_, Absolute, S>(system),
                        0xED => self.sbc::<_, Absolute, S>(system),
                        0xEE => self.inc::<_, Absolute, S>(system),
                        0xEF => self.bbs::<_, RelativeBitBranch, S>(system, 0x40),
                        0xF0 => self.branch::<_, Relative, S>(system, self.p & P_Z == P_Z),
                        0xF1 => self.sbc::<_, ZeroPageIndirectY, S>(system),
                        0xF2 => self.sbc::<_, ZeroPageIndirect, S>(system),
                        0xF3 => self.nop::<_, FastImplied, S>(system),
                        0xF4 => self.nop::<_, ZeroPageX, S>(system),
                        0xF5 => self.sbc::<_, ZeroPageX, S>(system),
                        0xF6 => self.inc::<_, ZeroPageX, S>(system),
                        0xF7 => self.smb::<_, ZeroPage, S>(system, 0x80),
                        0xF8 => self.sed(system),
                        0xF9 => self.sbc::<_, AbsoluteY, S>(system),
                        0xFA => self.plx(system),
                        0xFB => self.nop::<_, FastImplied, S>(system),
                        0xFC => self.nop::<_, Absolute, S>(system),
                        0xFD => self.sbc::<_, AbsoluteX, S>(system),
                        0xFE => self.inc::<_, AbsoluteXSlower, S>(system),
                        0xFF => self.bbs::<_, RelativeBitBranch, S>(system, 0x80),
                    }

                    return OPCODE_CYCLES[opcode as usize];
                }
            },
        }
        1
    }
}


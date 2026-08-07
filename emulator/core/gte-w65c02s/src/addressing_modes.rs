//! Addressing modes.

use super::{System, W65C02S};

pub trait AddressingMode {
    type Result;
    /// Fetch and process the operand, if any. (Do a spurious fetch if not.)
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> Self::Result;
}
pub trait Readable {
    /// Read the addressed data.
    fn read<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u8;
    fn read_spurious<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) { self.read(system, cpu); }
}
pub trait Writable {
    /// Write the addressed data.
    fn write<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S, data: u8);
}
pub trait RMWable : Readable + Writable {
    /// Perform the first read of a RMBx/SMBx instruction.
    fn read_locked<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u8;
    /// Perform the second read of a RMW instruction.
    fn read_locked_spurious<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S);
    /// Perform the write of a RMW instruction.
    fn write_locked<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S, data: u8);
}
pub trait HasEA {
    /// Return the effective address of this addressing mode.
    fn get_effective_address(&self) -> u16;
}
pub trait Branchable {
    /// Return the target address to branch to, if the branch is taken.
    fn get_branch_target<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u16;
}

pub struct SimpleEA { ea: u16 }
impl Readable for SimpleEA {
    fn read<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u8 {
        system.read(cpu, self.ea)
    }
    fn read_spurious<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) {
        // system.read_spurious(cpu, self.ea)
    }
}
impl Writable for SimpleEA {
    fn write<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S, data: u8) {
        system.write(cpu, self.ea, data)
    }
}
impl RMWable for SimpleEA {
    fn read_locked<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u8 {
        system.read_locked(cpu, self.ea)
    }
    fn read_locked_spurious<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) {
        // system.read_locked_spurious(cpu, self.ea);
    }
    fn write_locked<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S, data: u8) {
        system.write_locked(cpu, self.ea, data)
    }
}
impl HasEA for SimpleEA {
    fn get_effective_address(&self) -> u16 { self.ea }    
}

pub struct Implied {}
impl AddressingMode for Implied {
    type Result = ();
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> () {
        // system.read_operand_spurious(cpu, cpu.get_pc());
    }
}
pub struct FastImplied {}
impl AddressingMode for FastImplied {
    type Result = ();
    fn get_operand<S: System>(_: &mut S, _: &mut W65C02S) -> () {}
}

pub struct Absolute {}
impl AddressingMode for Absolute {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let ea_low = system.read_operand(cpu, pc);
        let pc = cpu.read_pc_postincrement();
        let ea_high = system.read_operand(cpu, pc);
        SimpleEA { ea: (ea_high as u16) << 8 | (ea_low as u16) }
    }
}

pub struct AbsoluteIndirect {}
impl AddressingMode for AbsoluteIndirect {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let addr_low = system.read_operand(cpu, pc);
        let pc = cpu.get_pc();
        let addr_high = system.read_operand(cpu, pc);
        let addr = (addr_high as u16) << 8 | (addr_low as u16);
        let pc = cpu.read_pc_postincrement();
        // system.read_spurious(cpu, pc);
        let ea_low = system.read_pointer(cpu, addr);
        let ea_high = system.read_pointer(cpu, addr.wrapping_add(1));
        SimpleEA { ea: (ea_high as u16) << 8 | (ea_low as u16) }
    }
}

pub struct AbsoluteX {}
impl AddressingMode for AbsoluteX {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base_low = system.read_operand(cpu, pc);
        let pc = cpu.read_pc_postincrement();
        let base_high = system.read_operand(cpu, pc);
        let base = (base_high as u16) << 8 | (base_low as u16);
        let ea = base.wrapping_add(cpu.get_x() as u16);
        if (ea & 0xFF00) != (base & 0xFF00) {
            let pc = cpu.get_pc().wrapping_sub(1);
            // system.read_spurious(cpu, pc);
        }
        SimpleEA { ea }
    }
}

pub struct AbsoluteXSlower {}
impl AddressingMode for AbsoluteXSlower {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base_low = system.read_operand(cpu, pc);
        let pc = cpu.read_pc_postincrement();
        let base_high = system.read_operand(cpu, pc);
        let base = (base_high as u16) << 8 | (base_low as u16);
        let ea = base.wrapping_add(cpu.get_x() as u16);
        if (ea & 0xFF00) != (base & 0xFF00) {
            let pc = cpu.get_pc().wrapping_sub(1);
            // system.read_spurious(cpu, pc);
        }
        else {
            // system.read_spurious(cpu, ea);
        }
        SimpleEA { ea }
    }
}

pub struct AbsoluteXIndirect {}
impl AddressingMode for AbsoluteXIndirect {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base_low = system.read_operand(cpu, pc);
        let pc = cpu.get_pc();
        let base_high = system.read_operand(cpu, pc);
        let base = (base_high as u16) << 8 | (base_low as u16);
        let addr = base.wrapping_add(cpu.get_x() as u16);
        let pc = cpu.read_pc_postincrement();
        // system.read_spurious(cpu, pc);
        let ea_low = system.read_pointer(cpu, addr);
        let ea_high = system.read_pointer(cpu, addr.wrapping_add(1));
        SimpleEA { ea: (ea_high as u16) << 8 | (ea_low as u16) }
    }
}

pub struct AbsoluteY {}
impl AddressingMode for AbsoluteY {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base_low = system.read_operand(cpu, pc);
        let pc = cpu.read_pc_postincrement();
        let base_high = system.read_operand(cpu, pc);
        let base = (base_high as u16) << 8 | (base_low as u16);
        let ea = base.wrapping_add(cpu.get_y() as u16);
        if (ea & 0xFF00) != (base & 0xFF00) {
            let pc = cpu.get_pc().wrapping_sub(1);
            // system.read_spurious(cpu, pc);
        }
        SimpleEA { ea }
    }
}

pub struct AbsoluteYSlower {}
impl AddressingMode for AbsoluteYSlower {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base_low = system.read_operand(cpu, pc);
        let pc = cpu.read_pc_postincrement();
        let base_high = system.read_operand(cpu, pc);
        let base = (base_high as u16) << 8 | (base_low as u16);
        let ea = base.wrapping_add(cpu.get_y() as u16);
        if (ea & 0xFF00) != (base & 0xFF00) {
            let pc = cpu.get_pc().wrapping_sub(1);
            // system.read_spurious(cpu, pc);
        }
        else {
            // system.read_spurious(cpu, ea);
        }
        SimpleEA { ea }
    }
}

pub struct ImpliedA {}
impl AddressingMode for ImpliedA {
    type Result = ImpliedA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> ImpliedA {
        // system.read_operand_spurious(cpu, cpu.get_pc());
        ImpliedA {}
    }
}
impl Readable for ImpliedA {
    fn read<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_a()
    }
    // fn read_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {}
}
impl Writable for ImpliedA {
    fn write<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_a(data)
    }
}
impl RMWable for ImpliedA {
    fn read_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_a()
    }
    fn read_locked_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {
    }
    fn write_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_a(data)
    }
}

pub struct ImpliedX {}
impl AddressingMode for ImpliedX {
    type Result = ImpliedX;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> ImpliedX {
        // system.read_operand_spurious(cpu, cpu.get_pc());
        ImpliedX {}
    }
}
impl Readable for ImpliedX {
    fn read<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_x()
    }
    fn read_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {}
}
impl Writable for ImpliedX {
    fn write<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_x(data)
    }
}
impl RMWable for ImpliedX {
    fn read_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_x()
    }
    fn read_locked_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {
    }
    fn write_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_x(data)
    }
}

pub struct ImpliedY {}
impl AddressingMode for ImpliedY {
    type Result = ImpliedY;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> ImpliedY {
        // system.read_operand_spurious(cpu, cpu.get_pc());
        ImpliedY {}
    }
}
impl Readable for ImpliedY {
    fn read<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_y()
    }
    fn read_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {}
}
impl Writable for ImpliedY {
    fn write<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_y(data)
    }
}
impl RMWable for ImpliedY {
    fn read_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S) -> u8 {
        cpu.get_y()
    }
    fn read_locked_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {
    }
    fn write_locked<S: System>(&mut self, _: &mut S, cpu: &mut W65C02S, data: u8) {
        cpu.set_y(data)
    }
}

pub struct Immediate {
    value: u8
}
impl AddressingMode for Immediate {
    type Result = Immediate;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> Immediate {
        let pc = cpu.read_pc_postincrement();
        let value = system.read_operand(cpu, pc);
        Immediate { value }
    }
}
impl Readable for Immediate {
    fn read<S: System>(&mut self, _: &mut S, _: &mut W65C02S) -> u8 {
        self.value
    }
}

pub struct Relative {
    target: u16
}
impl AddressingMode for Relative {
    type Result = Relative;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> Relative {
        let pc = cpu.read_pc_postincrement();
        let value = system.read_operand(cpu, pc) as i8;
        let target = cpu.get_pc().wrapping_add(value as u16);
        Relative { target }
    }
}
impl Branchable for Relative {
    fn get_branch_target<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u16 {
        // always burn one cycle
        // system.read_spurious(cpu, cpu.get_pc());
        if cpu.get_pc() & 0xFF00 != self.target & 0xFF00 {
            let old_irq_pending = cpu.irq_pending;
            cpu.check_irq_edge();
            cpu.irq_pending = cpu.irq_pending | old_irq_pending;
            // another cycle burns!
            // system.read_spurious(cpu, cpu.get_pc());
        }
        self.target
    }
}

pub struct RelativeBitBranch {
    data: u8,
    target: u16
}
impl AddressingMode for RelativeBitBranch {
    type Result = RelativeBitBranch;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> RelativeBitBranch {
        let pc = cpu.read_pc_postincrement();
        let addr = system.read_operand(cpu, pc);
        let data = system.read(cpu, addr as u16);
        // TODO: which value actually gets used?
        // system.read_spurious(cpu, addr as u16);
        let pc = cpu.read_pc_postincrement();
        let value = system.read_operand(cpu, pc) as i8;
        let target = cpu.get_pc().wrapping_add(value as u16);
        RelativeBitBranch { data, target }
    }
}
impl Readable for RelativeBitBranch {
    fn read<S: System>(&mut self, _: &mut S, _: &mut W65C02S) -> u8 {
        self.data
    }
}
impl Branchable for RelativeBitBranch {
    fn get_branch_target<S: System>(&mut self, system: &mut S, cpu: &mut W65C02S) -> u16 {
        // always burn one cycle
        // system.read_spurious(cpu, cpu.get_pc());
        if cpu.get_pc() & 0xFF00 != self.target & 0xFF00 {
            let old_irq_pending = cpu.irq_pending;
            cpu.check_irq_edge();
            cpu.irq_pending = cpu.irq_pending | old_irq_pending;
            // another cycle burns!
            // system.read_spurious(cpu, cpu.get_pc());
        }
        self.target
    }
}

pub struct ZeroPage {}
impl AddressingMode for ZeroPage {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let ea = system.read_operand(cpu, pc) as u16;
        SimpleEA { ea }
    }
}

pub struct ZeroPageIndirect {}
impl AddressingMode for ZeroPageIndirect {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let addr = system.read_operand(cpu, pc);
        let ea_low = system.read_pointer(cpu, addr as u16);
        let ea_high = system.read_pointer(cpu, addr.wrapping_add(1) as u16);
        SimpleEA { ea: (ea_high as u16) << 8 | (ea_low as u16) }
    }
}

pub struct ZeroPageX {}
impl AddressingMode for ZeroPageX {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base = system.read_operand(cpu, pc);
        let ea = base.wrapping_add(cpu.get_x()) as u16;
        // system.read_spurious(cpu, cpu.get_pc().wrapping_sub(1));
        SimpleEA { ea }
    }
}

pub struct ZeroPageXIndirect {}
impl AddressingMode for ZeroPageXIndirect {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base = system.read_operand(cpu, pc);
        let addr = base.wrapping_add(cpu.get_x());
        // system.read_spurious(cpu, cpu.get_pc().wrapping_sub(1));
        let ea_low = system.read_pointer(cpu, addr as u16);
        let ea_high = system.read_pointer(cpu, addr.wrapping_add(1) as u16);
        SimpleEA { ea: (ea_high as u16) << 8 | (ea_low as u16) }
    }
}

pub struct ZeroPageY {}
impl AddressingMode for ZeroPageY {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base = system.read_operand(cpu, pc);
        let ea = base.wrapping_add(cpu.get_y()) as u16;
        // system.read_spurious(cpu, cpu.get_pc().wrapping_sub(1));
        SimpleEA { ea }
    }
}

pub struct ZeroPageIndirectY {}
impl AddressingMode for ZeroPageIndirectY {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base = system.read_operand(cpu, pc);
        let addr_low = system.read_pointer(cpu, base as u16);
        let addr_high = system.read_pointer(cpu, base.wrapping_add(1) as u16);
        let addr = (addr_high as u16) << 8 | (addr_low as u16);
        let ea = addr.wrapping_add(cpu.get_y() as u16);
        if ea & 0xFF00 != addr & 0xFF00 {
            // system.read_spurious(cpu, base.wrapping_add(1) as u16);
        }
        SimpleEA { ea }
    }
}

pub struct ZeroPageIndirectYSlower {}
impl AddressingMode for ZeroPageIndirectYSlower {
    type Result = SimpleEA;
    fn get_operand<S: System>(system: &mut S, cpu: &mut W65C02S) -> SimpleEA {
        let pc = cpu.read_pc_postincrement();
        let base = system.read_operand(cpu, pc);
        let addr_low = system.read_pointer(cpu, base as u16);
        let addr_high = system.read_pointer(cpu, base.wrapping_add(1) as u16);
        let addr = (addr_high as u16) << 8 | (addr_low as u16);
        let ea = addr.wrapping_add(cpu.get_y() as u16);
        // system.read_spurious(cpu, base.wrapping_add(1) as u16);
        SimpleEA { ea }
    }
}

impl Readable for () {
    fn read<S: System>(&mut self, _: &mut S, _: &mut W65C02S) -> u8 {
        panic!("null read");
    }
    fn read_spurious<S: System>(&mut self, _: &mut S, _: &mut W65C02S) {}
}

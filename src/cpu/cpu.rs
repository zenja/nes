use super::{addr::AddrMode, spec::Spec};
use std::collections::HashMap;

use crate::bus::Bus;

#[allow(dead_code)]
pub struct Cpu {
    pc: u16,           // Program Counter
    sp: u8,            // Stack Pointer
    acc: u8,           // Accumulator
    reg_x: u8,         // Index Register X
    reg_y: u8,         // Index Register Y
    status: CpuStatus, // Processor Status

    cycles: u32, // Number of cycles remaining for this instruction

    bus: Bus,

    // Internal helpers
    opcode_to_spec: HashMap<u8, Spec>,
}

impl Cpu {
    fn new() -> Cpu {
        Cpu {
            pc: 0,
            sp: 0,
            acc: 0,
            reg_x: 0,
            reg_y: 0,
            status: CpuStatus::new(),
            cycles: 0,
            bus: Bus::new(),
            opcode_to_spec: super::spec::opcode_to_spec(),
        }
    }

    fn load_program(&mut self, program: Vec<u8>) {
        self.bus.write_batch(0x8000, program);
        self.pc = 0x8000;
    }

    fn reset(&mut self) {
        self.pc = 0;
        self.sp = 0xFD;
        self.acc = 0;
        self.reg_x = 0;
        self.reg_y = 0;
        self.status.reset();
        // Reset takes time
        self.cycles = 8;
    }

    // one cycle of execution
    fn tick(&mut self) {
        // if cycle is 0, it means a new instruction can be executed
        if self.cycles == 0 {
            // Always set the unused status flag bit to 1
            self.set_status(self::CpuStatusBit::U, true);

            let opcode = self.fetch_opcode();
            let Spec {
                opcode,
                addr_mode,
                base_cycles,
                inc_cycle_on_page_crossed,
                ..
            } = *self.opcode_to_spec.get(&opcode).unwrap();

            self.cycles = base_cycles as u32;

            let oprand_addr = self.fetch_oprand_addr(addr_mode, inc_cycle_on_page_crossed);
            self.execute_op(opcode, addr_mode, oprand_addr);

            // Always set the unused status flag bit to 1
            self.set_status(self::CpuStatusBit::U, true);
        }

        self.cycles -= 1;
    }

    fn fetch_opcode(&mut self) -> u8 {
        let opcode = self.bus.read(self.pc);
        self.pc += 1;
        opcode
    }

    fn fetch_oprand_addr(&mut self, addr_mode: AddrMode, inc_cycle_on_page_crossed: bool) -> u16 {
        use super::addr::AddrMode::*;

        let next_u8: u8 = self.bus.read(self.pc);
        let next_u16: u16 = self.read_u16(self.pc);
        let next_i8: i8 = i8::from_le_bytes([next_u8]);
        let addr = match addr_mode {
            Absolute => next_u16,
            AbsoluteX => {
                let addr = next_u16.wrapping_add(self.reg_x as u16);
                if addr & 0xFF00 != next_u16 & 0xFF00 && inc_cycle_on_page_crossed {
                    self.cycles += 1;
                }
                addr
            }
            AbsoluteY => {
                let addr = next_u16.wrapping_add(self.reg_y as u16);
                if addr & 0xFF00 != next_u16 & 0xFF00 && inc_cycle_on_page_crossed {
                    self.cycles += 1;
                }
                addr
            }
            ZeroPage => next_u8 as u16,
            ZeroPageX => (next_u8.wrapping_add(self.reg_x)) as u16,
            ZeroPageY => (next_u8.wrapping_add(self.reg_y)) as u16,
            Immediate => self.pc,
            // for relative addressing, handle additional cycles in instruction itself
            Relative => ((self.pc as i32) + Relative.size() as i32 + (next_i8 as i32)) as u16,
            Implicit => 0,
            Indirect => self.read_u16(next_u16),
            IndexedIndirect => self.read_u16((next_u8.wrapping_add(self.reg_x)) as u16),
            IndirectIndexed => {
                let addr = self
                    .read_u16(next_u8 as u16)
                    .wrapping_add(self.reg_y as u16);
                if addr & 0xFF00 != self.read_u16(next_u8 as u16) & 0xFF00
                    && inc_cycle_on_page_crossed
                {
                    self.cycles += 1
                }
                addr
            }
        };

        self.pc += addr_mode.size() as u16;

        addr
    }

    // return additional cycles caused by execution
    fn execute_op(
        &mut self,
        opcode: super::spec::Opcode,
        addr_mode: super::addr::AddrMode,
        oprand_addr: u16,
    ) {
        use self::CpuStatusBit::*;
        use super::addr::AddrMode::*;
        use super::spec::Opcode::*;

        fn handle_branching(oprand_addr: u16, cycles: &mut u32, pc: &mut u16) {
            *cycles += 1;

            if oprand_addr & 0xFF00 != *pc & 0xFF00 {
                *cycles += 1;
            }

            *pc = oprand_addr;
        }

        match opcode {
            ADC => {
                let oprand = self.bus.read(oprand_addr);
                let result: u8 = self
                    .acc
                    .wrapping_add(oprand)
                    .wrapping_add(self.get_status(C) as u8);
                let tmp = self.acc as u16 + oprand as u16 + self.get_status(C) as u16;
                self.set_status(C, tmp > 255);
                self.set_status(Z, result == 0);
                let overflow: bool = ((!((self.acc as u16) ^ (oprand as u16))
                    ^ ((self.acc as u16) ^ (tmp as u16)))
                    & 0x0080)
                    != 0;
                self.set_status(V, overflow);
                self.set_status(N, (tmp & 0x0080) != 0);
                self.acc = result;
            }
            SBC => {
                let oprand = self.bus.read(oprand_addr);
                let value = (oprand as u16) ^ 0x00FF;
                let tmp = self.acc as u16 + value + self.get_status(C) as u16;
                self.set_status(C, tmp & 0xFF00 != 0);
                self.set_status(Z, tmp & 0x00FF == 0);
                let overflow: bool = (tmp ^ (self.acc as u16)) & (tmp ^ value) & 0x0080 != 0;
                self.set_status(V, overflow);
                self.set_status(N, (tmp & 0x0080) != 0);
                self.acc = (tmp & 0x00FF) as u8;
            }
            AND => {
                let oprand = self.bus.read(oprand_addr);
                self.acc = self.acc & oprand;
                self.set_status(Z, self.acc == 0);
                self.set_status(N, (self.acc & 0x80) != 0);
            }
            ASL => {
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.bus.read(oprand_addr)
                };
                let tmp: u16 = (oprand << 1) as u16;
                self.set_status(C, tmp & 0xFF00 != 0);
                self.set_status(Z, tmp & 0x00FF == 0);
                self.set_status(N, (tmp & 0x0080) != 0);
                let result = (tmp & 0x00FF) as u8;
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.bus.write(oprand_addr, result);
                }
            }
            BCC => {
                if self.get_status(C) == false {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BCS => {
                if self.get_status(C) == true {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BEQ => {
                if self.get_status(Z) == true {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BIT => {
                let oprand = self.bus.read(oprand_addr);
                let tmp = oprand & self.acc;
                self.set_status(Z, tmp == 0);
                self.set_status(N, tmp & (1 << 7) != 0);
                self.set_status(V, tmp & (1 << 6) != 0);
            }
            BMI => {
                if self.get_status(N) == true {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BNE => {
                if self.get_status(Z) == false {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BPL => {
                if self.get_status(N) == false {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BRK => {
                // pc++;

                // SetFlag(I, 1);
                // write(0x0100 + stkp, (pc >> 8) & 0x00FF);
                // stkp--;
                // write(0x0100 + stkp, pc & 0x00FF);
                // stkp--;

                // SetFlag(B, 1);
                // write(0x0100 + stkp, status);
                // stkp--;
                // SetFlag(B, 0);

                // pc = (uint16_t)read(0xFFFE) | ((uint16_t)read(0xFFFF) << 8);
                self.pc += 1;

                self.set_status(I, true);
                self.bus
                    .write(0x0100 + self.sp as u16, ((self.pc >> 8) & 0x00FF) as u8);
                self.sp -= 1;
                self.bus
                    .write(0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
                self.sp -= 1;

                self.set_status(B, true);
                self.bus.write(0x0100 + self.sp as u16, self.status.bits);
                self.sp -= 1;
                self.set_status(B, false);

                self.pc = (self.bus.read(0xFFFE) as u16) | ((self.bus.read(0xFFFF) as u16) << 8);
            }
            BVC => {
                if self.get_status(N) == false {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            BVS => {
                if self.get_status(V) == true {
                    handle_branching(oprand_addr, &mut self.cycles, &mut self.pc);
                }
            }
            CLC => {
                self.set_status(C, false);
            }
            CLD => {
                self.set_status(D, false);
            }
            CLI => {
                self.set_status(I, false);
            }
            CLV => {
                self.set_status(V, false);
            }
            CMP => {
                let oprand = self.bus.read(oprand_addr);
                let result = self.acc.wrapping_sub(oprand);
                self.set_status(C, self.acc >= oprand);
                self.update_status_Z_N(result);
            }
            CPX => {
                let oprand = self.bus.read(oprand_addr);
                let result = self.reg_x.wrapping_sub(oprand);
                self.set_status(C, self.reg_x >= oprand);
                self.update_status_Z_N(result);
            }
            CPY => {
                let oprand = self.bus.read(oprand_addr);
                let result = self.reg_y.wrapping_sub(oprand);
                self.set_status(C, self.reg_y >= oprand);
                self.update_status_Z_N(result);
            }
            DEC => {
                let oprand = self.bus.read(oprand_addr);
                let result = oprand.wrapping_sub(1);
                self.bus.write(oprand_addr, result);
                self.update_status_Z_N(result);
            }
            DEX => {
                self.reg_x -= 1;
                self.update_status_Z_N(self.reg_x);
            }
            DEY => {
                self.reg_y -= 1;
                self.update_status_Z_N(self.reg_y);
            }
            EOR => {
                let oprand = self.bus.read(oprand_addr);
                let result = self.acc ^ oprand;
                self.acc = result;
                self.update_status_Z_N(result);
            }
            INC => {
                let oprand = self.bus.read(oprand_addr);
                let result = oprand.wrapping_add(1);
                self.bus.write(oprand_addr, result);
                self.update_status_Z_N(result);
            }
            INX => {
                self.reg_x += 1;
                self.update_status_Z_N(self.reg_x);
            }
            INY => {
                self.reg_y += 1;
                self.update_status_Z_N(self.reg_y);
            }
            JMP => {
                self.pc = oprand_addr;
            }
            JSR => {
                // pc--;

                // write(0x0100 + stkp, (pc >> 8) & 0x00FF);
                // stkp--;
                // write(0x0100 + stkp, pc & 0x00FF);
                // stkp--;

                // pc = addr_abs;
                self.pc -= 1;

                self.bus
                    .write(0x0100 + self.sp as u16, ((self.pc >> 8) & 0x00FF) as u8);
                self.sp -= 1;
                self.bus
                    .write(0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
                self.sp -= 1;

                self.pc = oprand_addr;
            }
            LDA => {
                let oprand = self.bus.read(oprand_addr);
                self.acc = oprand;
                self.update_status_Z_N(oprand);
            }
            LDX => {
                let oprand = self.bus.read(oprand_addr);
                self.reg_x = oprand;
                self.update_status_Z_N(oprand);
            }
            LDY => {
                let oprand = self.bus.read(oprand_addr);
                self.reg_y = oprand;
                self.update_status_Z_N(oprand);
            }
            LSR => {
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.bus.read(oprand_addr)
                };
                self.set_status(C, oprand & 0x01 == 1);
                let result = oprand >> 1;
                self.update_status_Z_N(result);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.bus.write(oprand_addr, result);
                }
            }
            NOP => {
                unimplemented!();
            }
            ORA => {
                let oprand = self.bus.read(oprand_addr);
                self.acc = self.acc | oprand;
                self.update_status_Z_N(self.acc);
            }
            PHA => {
                self.stack_push(self.acc);
            }
            PHP => {
                let mut cloned_status = self.status.clone();
                cloned_status.turn_on(B);
                cloned_status.turn_on(U);
                let result: u8 = cloned_status.bits;
                self.set_status(B, false);
                self.set_status(U, false);
                self.stack_push(result)
            }
            PLA => {
                self.acc = self.stack_pop();
                self.update_status_Z_N(self.acc);
            }
            PLP => {
                self.status.bits = self.stack_pop();
                self.set_status(U, true);
            }
            ROL => {
                // temp = (uint16_t)(fetched << 1) | GetFlag(C);
                // SetFlag(C, temp & 0xFF00);
                // SetFlag(Z, (temp & 0x00FF) == 0x0000);
                // SetFlag(N, temp & 0x0080);
                // if (lookup[opcode].addrmode == &olc6502::IMP)
                //     a = temp & 0x00FF;
                // else
                //     write(addr_abs, temp & 0x00FF);
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.bus.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((oprand << 1) as u16) | (c_bits as u16);
                self.set_status(C, tmp & 0xFF00 != 0);
                let result = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.bus.write(oprand_addr, result);
                }
            }
            ROR => {
                // temp = (uint16_t)(GetFlag(C) << 7) | (fetched >> 1);
                // SetFlag(C, fetched & 0x01);
                // SetFlag(Z, (temp & 0x00FF) == 0x00);
                // SetFlag(N, temp & 0x0080);
                // if (lookup[opcode].addrmode == &olc6502::IMP)
                // 	a = temp & 0x00FF;
                // else
                // 	write(addr_abs, temp & 0x00FF);
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.bus.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((c_bits << 7) as u16) | (oprand as u16 >> 1);
                let result = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.bus.write(oprand_addr, result);
                }
            }
            RTI => {
                self.status.bits = self.stack_pop();
                self.turn_off_status(B);
                self.turn_on_status(U);
                self.pc = self.stack_pop_u16();
            }
            RTS => {
                self.pc = self.stack_pop_u16().wrapping_add(1);
            }
            SEC => {
                self.turn_off_status(C);
            }
            SED => {
                self.turn_off_status(D);
            }
            SEI => {
                self.turn_off_status(I);
            }
            STA => {
                self.bus.write(oprand_addr, self.acc);
            }
            STX => {
                self.bus.write(oprand_addr, self.reg_x);
            }
            STY => {
                self.bus.write(oprand_addr, self.reg_y);
            }
            TAX => {
                self.reg_x = self.acc;
                self.update_status_Z_N(self.reg_x);
            }
            TAY => {
                self.reg_y = self.acc;
                self.update_status_Z_N(self.reg_y);
            }
            TSX => {
                self.reg_x = self.sp;
                self.update_status_Z_N(self.reg_x);
            }
            TXA => {
                self.acc = self.reg_x;
                self.update_status_Z_N(self.acc);
            }
            TXS => {
                self.sp = self.reg_x;
            }
            TYA => {
                self.acc = self.reg_y;
                self.update_status_Z_N(self.acc);
            }
        }
    }

    fn read_u16(&self, addr: u16) -> u16 {
        let a = self.bus.read(addr);
        let b = self.bus.read(addr + 1);
        u16::from_le_bytes([a, b])
    }

    fn set_status(&mut self, bit: CpuStatusBit, set: bool) {
        self.status.set(bit, set);
    }

    fn get_status(&self, bit: CpuStatusBit) -> bool {
        self.status.get(bit)
    }

    fn turn_on_status(&mut self, bit: CpuStatusBit) {
        self.status.turn_on(bit);
    }

    fn turn_off_status(&mut self, bit: CpuStatusBit) {
        self.status.turn_off(bit);
    }

    fn update_status_Z_N(&mut self, result: u8) {
        use self::CpuStatusBit::{N, Z};
        self.set_status(Z, result == 0);
        self.set_status(N, result & 0b1000_0000 != 0);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.read((0x0100 as u16) + self.sp as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.bus.write((0x0100 as u16) + self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1)
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        hi << 8 | lo
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct CpuStatus {
    bits: u8,
}

#[derive(Clone, Copy)]
enum CpuStatusBit {
    C,
    Z,
    I,
    D,
    B,
    U,
    V,
    N,
}

impl CpuStatusBit {
    fn bit_offset(self) -> u8 {
        match self {
            Self::C => 0,
            Self::Z => 1,
            Self::I => 2,
            Self::D => 3,
            Self::B => 4,
            Self::U => 5,
            Self::V => 6,
            Self::N => 7,
        }
    }
}

impl CpuStatus {
    fn new() -> CpuStatus {
        CpuStatus { bits: 0 }
    }

    fn reset(&mut self) {
        self.bits = 0;
    }

    fn set_from_bits(&mut self, bits: u8) {
        self.bits = bits;
    }

    fn get(&self, bit: CpuStatusBit) -> bool {
        self.bits & (1 << bit.bit_offset()) != 0
    }

    fn set(&mut self, bit: CpuStatusBit, set: bool) {
        if set {
            self.turn_on(bit);
        } else {
            self.turn_off(bit);
        }
    }

    fn turn_on(&mut self, bit: CpuStatusBit) {
        self.bits = self.bits | (1 << bit.bit_offset());
    }

    fn turn_off(&mut self, bit: CpuStatusBit) {
        self.bits = self.bits & !(1 << bit.bit_offset());
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_reset_cpu() -> Cpu {
        let mut cpu = Cpu::new();
        cpu.reset();
        cpu
    }

    fn new_cpu_with_program(program: Vec<u8>) -> Cpu {
        let mut cpu = Cpu::new();
        cpu.reset();
        cpu.load_program(program);
        cpu
    }

    #[test]
    fn test_load_program() {
        let mut cpu = new_reset_cpu();
        cpu.load_program(vec![0x01, 0x23, 0x34]);
        assert_eq!(cpu.bus.read(cpu.pc), 0x01);
        assert_eq!(cpu.bus.read(cpu.pc + 1), 0x23);
        assert_eq!(cpu.bus.read(cpu.pc + 2), 0x34);
        assert_eq!(cpu.bus.read(cpu.pc + 3), 0x00);
    }

    #[test]
    fn test_fetch_opcode() {
        let mut cpu = new_reset_cpu();
        let program: Vec<u8> = vec![0x8d, 0x00, 0xc0]; // STA $c000
        cpu.load_program(program);
        let origin_pc = cpu.pc;
        let opcode = cpu.fetch_opcode();
        assert_eq!(opcode, 0x8d);
        assert_eq!(cpu.pc, origin_pc + 1);
    }

    #[test]
    fn test_fetch_oprand_addr() {
        fn assert_addr_eq(actual: u16, expected: u16) {
            assert_eq!(
                actual, expected,
                "Expected: 0x{:04X?}; Actual: 0x{:04X}",
                expected, actual
            );
        }

        // STA $c000
        let mut cpu = new_cpu_with_program(vec![0x8d, 0x00, 0xc0]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Absolute, false);
        let expected: u16 = 0xC000;
        assert_addr_eq(actual, expected);

        // STA $0200,X
        let mut cpu = new_cpu_with_program(vec![0x9d, 0x00, 0x02]);
        cpu.fetch_opcode();
        cpu.reg_x = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::AbsoluteX, false);
        let expected: u16 = 0x0201;
        assert_addr_eq(actual, expected);

        // STA $0200,Y
        let mut cpu = new_cpu_with_program(vec![0x9d, 0x00, 0x02]);
        cpu.fetch_opcode();
        cpu.reg_y = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::AbsoluteY, false);
        let expected: u16 = 0x0201;
        assert_addr_eq(actual, expected);

        // STA $c0
        let mut cpu = new_cpu_with_program(vec![0x85, 0xc0]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPage, false);
        let expected: u16 = 0x00c0;
        assert_addr_eq(actual, expected);

        // STA $c0,X
        let mut cpu = new_cpu_with_program(vec![0x95, 0xc0]);
        cpu.fetch_opcode();
        cpu.reg_x = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPageX, false);
        let expected: u16 = 0x00c1;
        assert_addr_eq(actual, expected);

        // LDX $c0,Y
        let mut cpu = new_cpu_with_program(vec![0xb6, 0xc0]);
        cpu.fetch_opcode();
        cpu.reg_y = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPageY, false);
        let expected: u16 = 0x00c1;
        assert_addr_eq(actual, expected);

        // LDX #$c0
        let mut cpu = new_cpu_with_program(vec![0xa2, 0xc0]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Immediate, false);
        let expected: u16 = 0x8001;
        assert_addr_eq(actual, expected);

        // BNE not_equal
        // not_equal: BRK
        let mut cpu = new_cpu_with_program(vec![0xd0, 0x00, 0x00]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Relative, false);
        let expected: u16 = 0x8002;
        assert_addr_eq(actual, expected);

        // INX
        let mut cpu = new_cpu_with_program(vec![0xe8]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Implicit, false);
        let expected: u16 = 0;
        assert_addr_eq(actual, expected);

        // JMP ($00f0)
        let mut cpu = new_cpu_with_program(vec![0x6c, 0xf0, 0x00]);
        cpu.bus.write(0x00f0, 0x12);
        cpu.bus.write(0x00f1, 0x34);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Indirect, false);
        let expected: u16 = 0x3412;
        assert_addr_eq(actual, expected);

        // LDA ($c0,X)
        let mut cpu = new_cpu_with_program(vec![0xa1, 0xc0]);
        cpu.bus.write(0x00c1, 0x12);
        cpu.bus.write(0x00c2, 0x34);
        cpu.reg_x = 1;
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::IndexedIndirect, false);
        let expected: u16 = 0x3412;
        assert_addr_eq(actual, expected);

        // LDA ($c0),Y
        let mut cpu = new_cpu_with_program(vec![0xb1, 0xc0]);
        cpu.bus.write(0x00c0, 0x12);
        cpu.bus.write(0x00c1, 0x34);
        cpu.reg_y = 1;
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::IndirectIndexed, false);
        let expected: u16 = 0x3413;
        assert_addr_eq(actual, expected);
    }

    #[test]
    fn test_cpu_status() {
        use super::CpuStatusBit::*;

        let mut status = CpuStatus::new();
        assert_eq!(status.bits, 0b0000_0000);

        status.set(C, true);
        status.turn_on(U);
        assert_eq!(status.bits, 0b0010_0001);

        status.turn_off(U);
        assert_eq!(status.bits, 0b0000_0001);
    }
}

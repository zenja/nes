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
    fn new_nes_cpu() -> Cpu {
        Cpu {
            pc: 0,
            sp: 0,
            acc: 0,
            reg_x: 0,
            reg_y: 0,
            status: CpuStatus::new(),
            cycles: 0,
            bus: Bus::new_nes_bus(),
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
            let opcode = self.fetch_opcode();
            let Spec {
                opcode,
                addr_mode,
                base_cycles,
                inc_cycle_on_page_crossed,
                ..
            } = self.opcode_to_spec.get(&opcode).unwrap();

            let addr = self.fetch_oprand_addr(*addr_mode);

            // TODO

            // update cycles
            // TODO
        }

        self.cycles -= 1;
    }

    fn fetch_opcode(&mut self) -> u8 {
        let opcode = self.bus.read(self.pc);
        self.pc += 1;
        opcode
    }

    fn fetch_oprand_addr(&mut self, addr_mode: AddrMode) -> u16 {
        use super::addr::AddrMode::*;

        let next_u8: u8 = self.bus.read(self.pc);
        let next_u16: u16 = self.read_u16(self.pc);
        let next_i8: i8 = i8::from_le_bytes([next_u8]);
        let addr = match addr_mode {
            Absolute => next_u16,
            AbsoluteX => next_u16 + self.reg_x as u16,
            AbsoluteY => next_u16 + self.reg_y as u16,
            ZeroPage => next_u8 as u16,
            ZeroPageX => (next_u8 + self.reg_x) as u16,
            ZeroPageY => (next_u8 + self.reg_y) as u16,
            Immediate => self.pc,
            Relative => ((self.pc as i32) + Relative.size() as i32 + (next_i8 as i32)) as u16,
            Implicit => 0,
            Indirect => self.read_u16(next_u16),
            IndexedIndirect => self.read_u16((next_u8 + self.reg_x) as u16),
            IndirectIndexed => self.read_u16(next_u8 as u16) + self.reg_y as u16,
        };
        self.pc += addr_mode.size() as u16;
        addr
    }

    fn execute_op(&mut self) {
        // TODO
    }

    fn read_u16(&self, addr: u16) -> u16 {
        let a = self.bus.read(addr);
        let b = self.bus.read(addr + 1);
        u16::from_le_bytes([a, b])
    }
}

#[allow(dead_code)]
struct CpuStatus {
    n: bool,
    v: bool,
    b: bool,
    d: bool,
    i: bool,
    z: bool,
    c: bool,
}

impl CpuStatus {
    fn new() -> CpuStatus {
        CpuStatus {
            n: false,
            v: false,
            b: false,
            d: false,
            i: false,
            z: false,
            c: false,
        }
    }

    fn reset(&mut self) {
        self.n = false;
        self.v = false;
        self.b = false;
        self.d = false;
        self.i = false;
        self.z = false;
        self.c = false;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_reset_cpu() -> Cpu {
        let mut cpu = Cpu::new_nes_cpu();
        cpu.reset();
        cpu
    }

    fn new_cpu_with_program(program: Vec<u8>) -> Cpu {
        let mut cpu = Cpu::new_nes_cpu();
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
        let actual = cpu.fetch_oprand_addr(AddrMode::Absolute);
        let expected: u16 = 0xC000;
        assert_addr_eq(actual, expected);

        // STA $0200,X
        let mut cpu = new_cpu_with_program(vec![0x9d, 0x00, 0x02]);
        cpu.fetch_opcode();
        cpu.reg_x = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::AbsoluteX);
        let expected: u16 = 0x0201;
        assert_addr_eq(actual, expected);

        // STA $0200,Y
        let mut cpu = new_cpu_with_program(vec![0x9d, 0x00, 0x02]);
        cpu.fetch_opcode();
        cpu.reg_y = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::AbsoluteY);
        let expected: u16 = 0x0201;
        assert_addr_eq(actual, expected);

        // STA $c0
        let mut cpu = new_cpu_with_program(vec![0x85, 0xc0]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPage);
        let expected: u16 = 0x00c0;
        assert_addr_eq(actual, expected);

        // STA $c0,X
        let mut cpu = new_cpu_with_program(vec![0x95, 0xc0]);
        cpu.fetch_opcode();
        cpu.reg_x = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPageX);
        let expected: u16 = 0x00c1;
        assert_addr_eq(actual, expected);

        // LDX $c0,Y
        let mut cpu = new_cpu_with_program(vec![0xb6, 0xc0]);
        cpu.fetch_opcode();
        cpu.reg_y = 0x01;
        let actual = cpu.fetch_oprand_addr(AddrMode::ZeroPageY);
        let expected: u16 = 0x00c1;
        assert_addr_eq(actual, expected);

        // LDX #$c0
        let mut cpu = new_cpu_with_program(vec![0xa2, 0xc0]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Immediate);
        let expected: u16 = 0x8001;
        assert_addr_eq(actual, expected);

        // BNE not_equal
        // not_equal: BRK
        let mut cpu = new_cpu_with_program(vec![0xd0, 0x00, 0x00]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Relative);
        let expected: u16 = 0x8002;
        assert_addr_eq(actual, expected);

        // INX
        let mut cpu = new_cpu_with_program(vec![0xe8]);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Implicit);
        let expected: u16 = 0;
        assert_addr_eq(actual, expected);

        // JMP ($00f0)
        let mut cpu = new_cpu_with_program(vec![0x6c, 0xf0, 0x00]);
        cpu.bus.write(0x00f0, 0x12);
        cpu.bus.write(0x00f1, 0x34);
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::Indirect);
        let expected: u16 = 0x3412;
        assert_addr_eq(actual, expected);

        // LDA ($c0,X)
        let mut cpu = new_cpu_with_program(vec![0xa1, 0xc0]);
        cpu.bus.write(0x00c1, 0x12);
        cpu.bus.write(0x00c2, 0x34);
        cpu.reg_x = 1;
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::IndexedIndirect);
        let expected: u16 = 0x3412;
        assert_addr_eq(actual, expected);

        // LDA ($c0),Y
        let mut cpu = new_cpu_with_program(vec![0xb1, 0xc0]);
        cpu.bus.write(0x00c0, 0x12);
        cpu.bus.write(0x00c1, 0x34);
        cpu.reg_y = 1;
        cpu.fetch_opcode();
        let actual = cpu.fetch_oprand_addr(AddrMode::IndirectIndexed);
        let expected: u16 = 0x3413;
        assert_addr_eq(actual, expected);
    }
}

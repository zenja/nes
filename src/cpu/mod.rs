pub mod addr;
pub mod assembler;
pub mod spec;
pub mod trace;

use std::collections::HashMap;

use crate::bus::Bus;
use addr::AddrMode;
use spec::Spec;

#[allow(dead_code)]
pub struct CPU {
    pub pc: u16,       // Program Counter
    sp: u8,            // Stack Pointer
    acc: u8,           // Accumulator
    reg_x: u8,         // Index Register X
    reg_y: u8,         // Index Register Y
    status: CPUStatus, // Processor Status

    cycles: u32,       // Number of cycles remaining for this instruction
    total_cycles: u32, // Number of total cycles this CPU has executed

    pub bus: Bus,

    // Internal helpers
    opcode_to_spec: HashMap<u8, Spec>,
}

impl CPU {
    pub fn new(bus: Bus) -> CPU {
        CPU {
            pc: 0x8000,
            sp: 0,
            acc: 0,
            reg_x: 0,
            reg_y: 0,
            status: CPUStatus::new(),
            cycles: 0,
            total_cycles: 0,
            bus: bus,
            opcode_to_spec: spec::opcode_to_spec(),
        }
    }

    pub fn reset(&mut self) {
        self.pc = self.read_u16(0xFFFC);
        self.sp = 0xFD;
        self.acc = 0;
        self.reg_x = 0;
        self.reg_y = 0;
        self.status.reset();
        self.status.set(CPUStatusBit::I, true);
        self.status.set(CPUStatusBit::U, true);

        // Reset takes time
        self.cycles = 7;
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F: FnMut(&mut CPU)>(&mut self, mut callback: F) {
        loop {
            if self.bus.has_nmi() {
                self.cycles = self.nmi();
                self.bus.reset_nmi();
            }

            let should_callback = self.cycles == 0;
            if should_callback {
                callback(self);
            }

            self.tick();
            self.bus.cpu_tick();
        }
    }

    fn execute_next_instruction(&mut self) {
        // Always set the unused status flag bit to 1
        self.set_status(self::CPUStatusBit::U, true);

        let inst = self.fetch_next_instruction();
        self.cycles = inst.cycles as u32;
        self.execute_inst(inst);

        // Always set the unused status flag bit to 1
        self.set_status(self::CPUStatusBit::U, true);
    }

    // one cycle of execution
    fn tick(&mut self) {
        // if cycle is 0, it means a new instruction can be executed
        if self.cycles == 0 {
            self.execute_next_instruction();
        }

        self.cycles -= 1;
        self.total_cycles += 1;
    }

    fn fetch_next_instruction(&mut self) -> Instruction {
        let opcode_byte = self.read(self.pc);
        self.pc += 1;
        let spec = *self.opcode_to_spec.get(&opcode_byte).unwrap();
        let (oprand_addr, additional_cycles) =
            self.peak_oprand_addr_and_cycles(spec.addr_mode, spec.inc_cycle_on_page_crossed);
        self.pc += spec.addr_mode.size() as u16;
        Instruction {
            opcode_byte,
            oprand_addr,
            spec,
            cycles: (&spec.base_cycles + additional_cycles) as usize,
        }
    }

    // fetch next instruction, but keep CPU state unchanged
    fn peak_next_instruction(&mut self) -> Instruction {
        let pc = self.pc;
        let inst = self.fetch_next_instruction();
        self.pc = pc;
        inst
    }

    // return (oprand addr, cycles to advance)
    fn peak_oprand_addr_and_cycles(
        &mut self,
        addr_mode: AddrMode,
        inc_cycle_on_page_crossed: bool,
    ) -> (u16, u8) {
        use addr::AddrMode::*;

        let next_u8: u8 = self.read(self.pc);
        let next_u16: u16 = self.read_u16(self.pc);
        let next_i8: i8 = i8::from_le_bytes([next_u8]);
        match addr_mode {
            Absolute => (next_u16, 0u8),
            AbsoluteX => {
                let addr = next_u16.wrapping_add(self.reg_x as u16);
                let cycles = if addr & 0xFF00 != next_u16 & 0xFF00 && inc_cycle_on_page_crossed {
                    1u8
                } else {
                    0u8
                };
                (addr, cycles)
            }
            AbsoluteY => {
                let addr = next_u16.wrapping_add(self.reg_y as u16);
                let cycles = if addr & 0xFF00 != next_u16 & 0xFF00 && inc_cycle_on_page_crossed {
                    1u8
                } else {
                    0u8
                };
                (addr, cycles)
            }
            ZeroPage => (next_u8 as u16, 0u8),
            ZeroPageX => ((next_u8.wrapping_add(self.reg_x)) as u16, 0u8),
            ZeroPageY => ((next_u8.wrapping_add(self.reg_y)) as u16, 0u8),
            Immediate => (self.pc, 0u8),
            // for relative addressing, handle additional cycles in instruction itself
            Relative => (
                ((self.pc as i32) + Relative.size() as i32 + (next_i8 as i32)) as u16,
                0u8,
            ),
            Implicit => (0u16, 0u8),
            Indirect => (self.read_u16(next_u16), 0u8),
            IndexedIndirect => {
                let indexed = next_u8.wrapping_add(self.reg_x);
                let addr: u16 = if indexed == 0xFF {
                    self.read_u16(indexed as u16);
                    let a = self.read(0x00FF);
                    let b = self.read(0x0000);
                    u16::from_le_bytes([a, b])
                } else {
                    self.read_u16(indexed as u16)
                };
                (addr, 0u8)
            }
            IndirectIndexed => {
                let addr_before_add_y: u16 = if next_u8 == 0xFF {
                    let a = self.read(0x00FF);
                    let b = self.read(0x0000);
                    u16::from_le_bytes([a, b])
                } else {
                    self.read_u16(next_u8 as u16)
                };
                let addr = addr_before_add_y.wrapping_add(self.reg_y as u16);
                let cycles = if addr & 0xFF00 != self.read_u16(next_u8 as u16) & 0xFF00
                    && inc_cycle_on_page_crossed
                {
                    1
                } else {
                    0
                };
                (addr, cycles)
            }
        }
    }

    fn execute_inst(&mut self, inst: Instruction) {
        use self::CPUStatusBit::*;
        use addr::AddrMode::*;
        use spec::Opcode::*;

        fn handle_branching(oprand_addr: u16, cycles: &mut u32, pc: &mut u16) {
            *cycles += 1;

            if oprand_addr & 0xFF00 != *pc & 0xFF00 {
                *cycles += 1;
            }

            *pc = oprand_addr;
        }

        let addr_mode = inst.spec.addr_mode;
        let oprand_addr = inst.oprand_addr;
        let oprand = self.read(oprand_addr);

        match inst.spec.opcode {
            ADC => {
                let result: u8 = self
                    .acc
                    .wrapping_add(oprand)
                    .wrapping_add(self.get_status(C) as u8);
                let tmp = self.acc as u16 + oprand as u16 + self.get_status(C) as u16;
                self.set_status(C, tmp > 0xFF);
                self.set_status(Z, result == 0);
                let overflow: bool = ((result as u16) ^ (oprand as u16))
                    & ((self.acc as u16) ^ (result as u16))
                    & 0x0080
                    != 0;
                self.set_status(V, overflow);
                self.set_status(N, (tmp & 0x0080) != 0);
                self.acc = result;
            }
            SBC => {
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
                self.acc = self.acc & oprand;
                self.set_status(Z, self.acc == 0);
                self.set_status(N, (self.acc & 0x80) != 0);
            }
            ASL => {
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                let tmp: u16 = (oprand as u16) << 1;
                self.set_status(C, oprand & (1 << 7) != 0);
                self.set_status(Z, tmp & 0x00FF == 0);
                self.set_status(N, (tmp & 0x0080) != 0);
                let result = (tmp & 0x00FF) as u8;
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
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
                let tmp = oprand & self.acc;
                self.set_status(Z, tmp == 0);
                self.set_status(N, oprand & (1 << 7) != 0);
                self.set_status(V, oprand & (1 << 6) != 0);
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
                    .cpu_write(0x0100 + self.sp as u16, ((self.pc >> 8) & 0x00FF) as u8);
                self.sp -= 1;
                self.bus
                    .cpu_write(0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
                self.sp -= 1;

                self.set_status(B, true);
                self.bus
                    .cpu_write(0x0100 + self.sp as u16, self.status.bits);
                self.sp -= 1;
                self.set_status(B, false);

                self.pc = (self.read(0xFFFE) as u16) | ((self.read(0xFFFF) as u16) << 8);
            }
            BVC => {
                if self.get_status(V) == false {
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
                let result = self.acc.wrapping_sub(oprand);
                self.set_status(C, self.acc >= oprand);
                self.update_status_Z_N(result);
            }
            CPX => {
                let result = self.reg_x.wrapping_sub(oprand);
                self.set_status(C, self.reg_x >= oprand);
                self.update_status_Z_N(result);
            }
            CPY => {
                let result = self.reg_y.wrapping_sub(oprand);
                self.set_status(C, self.reg_y >= oprand);
                self.update_status_Z_N(result);
            }
            DEC => {
                let result = oprand.wrapping_sub(1);
                self.write(oprand_addr, result);
                self.update_status_Z_N(result);
            }
            DEX => {
                self.reg_x = self.reg_x.wrapping_sub(1);
                self.update_status_Z_N(self.reg_x);
            }
            DEY => {
                self.reg_y = self.reg_y.wrapping_sub(1);
                self.update_status_Z_N(self.reg_y);
            }
            EOR => {
                let result = self.acc ^ oprand;
                self.acc = result;
                self.update_status_Z_N(result);
            }
            INC => {
                let result = oprand.wrapping_add(1);
                self.write(oprand_addr, result);
                self.update_status_Z_N(result);
            }
            INX => {
                self.reg_x = self.reg_x.wrapping_add(1);
                self.update_status_Z_N(self.reg_x);
            }
            INY => {
                self.reg_y = self.reg_y.wrapping_add(1);
                self.update_status_Z_N(self.reg_y);
            }
            JMP => {
                // Caveat:
                // AN INDIRECT JUMP MUST NEVER USE A VECTOR
                // BEGINNING ON THE LAST BYTE OF A PAGE
                // Ref:http://www.6502.org/tutorials/6502opcodes.html#JMP
                let addr_before_indirect: u16 =
                    self.read_u16(self.pc - inst.spec.addr_mode.size() as u16);
                let oprand_addr: u16 = if let AddrMode::Indirect = inst.spec.addr_mode {
                    let a_addr = addr_before_indirect;
                    let b_addr = if a_addr & 0x00FF == 0x00FF {
                        a_addr & 0xFF00
                    } else {
                        addr_before_indirect.wrapping_add(1)
                    };
                    let a = self.read(a_addr);
                    let b = self.read(b_addr);
                    u16::from_le_bytes([a, b])
                } else {
                    inst.oprand_addr
                };
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
                    .cpu_write(0x0100 + self.sp as u16, ((self.pc >> 8) & 0x00FF) as u8);
                self.sp -= 1;
                self.bus
                    .cpu_write(0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
                self.sp -= 1;

                self.pc = oprand_addr;
            }
            LDA => {
                self.acc = oprand;
                self.update_status_Z_N(oprand);
            }
            LDX => {
                self.reg_x = oprand;
                self.update_status_Z_N(oprand);
            }
            LDY => {
                self.reg_y = oprand;
                self.update_status_Z_N(oprand);
            }
            LSR => {
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                self.set_status(C, oprand & 0x01 == 1);
                let result = oprand >> 1;
                self.update_status_Z_N(result);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
                }
            }
            NOP => {
                // do nothing
            }
            ORA => {
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
                self.set_status(B, false);
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
                    self.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((oprand << 1) as u16) | (c_bits as u16);
                self.set_status(C, tmp & 0xFF00 != 0);
                let result = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result);
                self.set_status(C, oprand & (1 << 7) != 0);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
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
                    self.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((c_bits << 7) as u16) | (oprand as u16 >> 1);
                let result = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result);
                self.set_status(C, oprand & 1 != 0);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
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
                self.turn_on_status(C);
            }
            SED => {
                self.turn_on_status(D);
            }
            SEI => {
                self.turn_on_status(I);
            }
            STA => {
                self.write(oprand_addr, self.acc);
            }
            STX => {
                self.write(oprand_addr, self.reg_x);
            }
            STY => {
                self.write(oprand_addr, self.reg_y);
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

            // ---------- Unofficial Opcodes ----------
            // Ref: https://wiki.nesdev.com/w/index.php/Programming_with_unofficial_opcodes
            LAX => {
                // LAX is shortcut for LDA value then TAX
                self.acc = oprand;
                self.reg_x = self.acc;
                self.update_status_Z_N(self.acc);
            }
            SAX => {
                // Stores the bitwise AND of A and X.
                // As with STA and STX, no flags are affected.
                self.write(oprand_addr, self.acc & self.reg_x);
            }
            DCP => {
                // Equivalent to DEC value then CMP value
                let result = oprand.wrapping_sub(1);
                self.write(oprand_addr, result);
                self.set_status(C, self.acc >= result);
                self.update_status_Z_N(self.acc.wrapping_sub(result));
            }
            ISB => {
                // Equivalent to INC value then SBC value
                let result = oprand.wrapping_add(1);
                self.write(oprand_addr, result);
                self.update_status_Z_N(result);

                let value = (result as u16) ^ 0x00FF;
                let tmp = self.acc as u16 + value + self.get_status(C) as u16;
                self.set_status(C, tmp & 0xFF00 != 0);
                self.set_status(Z, tmp & 0x00FF == 0);
                let overflow: bool = (tmp ^ (self.acc as u16)) & (tmp ^ value) & 0x0080 != 0;
                self.set_status(V, overflow);
                self.set_status(N, (tmp & 0x0080) != 0);
                self.acc = (tmp & 0x00FF) as u8;
            }
            SLO => {
                // Equivalent to ASL value then ORA value
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                let tmp: u16 = (oprand as u16) << 1;
                self.set_status(C, oprand & (1 << 7) != 0);
                self.set_status(Z, tmp & 0x00FF == 0);
                self.set_status(N, (tmp & 0x0080) != 0);
                let result = (tmp & 0x00FF) as u8;
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
                }

                self.acc = self.acc | result;
                self.update_status_Z_N(self.acc);
            }
            RLA => {
                // Equivalent to ROL value then AND value
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((oprand << 1) as u16) | (c_bits as u16);
                self.set_status(C, tmp & 0xFF00 != 0);
                let result = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result);
                self.set_status(C, oprand & (1 << 7) != 0);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
                }

                self.acc = self.acc & result;
                self.set_status(Z, self.acc == 0);
                self.set_status(N, (self.acc & 0x80) != 0);
            }
            SRE => {
                // Equivalent to LSR value then EOR value
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                self.set_status(C, oprand & 0x01 == 1);
                let mut result = oprand >> 1;
                self.update_status_Z_N(result);
                if let Implicit = addr_mode {
                    self.acc = result;
                } else {
                    self.write(oprand_addr, result);
                }

                result = self.acc ^ result;
                self.acc = result;
                self.update_status_Z_N(result);
            }
            RRA => {
                // Equivalent to ROR value then ADC value
                let oprand = if let Implicit = addr_mode {
                    self.acc
                } else {
                    self.read(oprand_addr)
                };
                let c_bits: u8 = if self.get_status(C) { 1 << 0 } else { 0 };
                let tmp: u16 = ((c_bits << 7) as u16) | (oprand as u16 >> 1);
                let result_ror = (tmp & 0x00FF) as u8;
                self.update_status_Z_N(result_ror);
                self.set_status(C, oprand & 1 != 0);
                if let Implicit = addr_mode {
                    self.acc = result_ror;
                } else {
                    self.write(oprand_addr, result_ror);
                }

                let result_adc: u8 = self
                    .acc
                    .wrapping_add(result_ror)
                    .wrapping_add(self.get_status(C) as u8);
                let tmp = self.acc as u16 + result_ror as u16 + self.get_status(C) as u16;
                self.set_status(C, tmp > 0xFF);
                self.set_status(Z, result_adc == 0);
                let overflow: bool = ((result_adc as u16) ^ (result_ror as u16))
                    & ((self.acc as u16) ^ (result_adc as u16))
                    & 0x0080
                    != 0;
                self.set_status(V, overflow);
                self.set_status(N, (tmp & 0x0080) != 0);
                self.acc = result_adc;
            }
        }
    }

    // return: number of cycles of nmi (always 8)
    fn nmi(&mut self) -> u32 {
        // write(0x0100 + stkp, (pc >> 8) & 0x00FF);
        // stkp--;
        // write(0x0100 + stkp, pc & 0x00FF);
        // stkp--;

        // SetFlag(B, 0);
        // SetFlag(U, 1);
        // SetFlag(I, 1);
        // write(0x0100 + stkp, status);
        // stkp--;

        // addr_abs = 0xFFFA;
        // uint16_t lo = read(addr_abs + 0);
        // uint16_t hi = read(addr_abs + 1);
        // pc = (hi << 8) | lo;

        // cycles = 8;

        use self::CPUStatusBit::*;

        self.bus
            .cpu_write(0x0100 + self.sp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.sp -= 1;
        self.bus
            .cpu_write(0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
        self.sp -= 1;

        self.set_status(B, false);
        self.set_status(U, true);
        self.set_status(I, true);
        self.bus
            .cpu_write(0x0100 + self.sp as u16, self.status.bits);
        self.sp -= 1;

        let addr_abs: u16 = 0xFFFA;
        let lo: u16 = self.bus.cpu_read(addr_abs + 0) as u16;
        let hi: u16 = self.bus.cpu_read(addr_abs + 1) as u16;
        self.pc = (hi << 8) | lo;

        // 8 cycles
        8
    }

    fn read(&mut self, addr: u16) -> u8 {
        self.bus.cpu_read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.bus.cpu_write(addr, value);
    }

    fn read_u16(&mut self, addr: u16) -> u16 {
        let a = self.read(addr);
        let b = self.read(addr + 1);
        u16::from_le_bytes([a, b])
    }

    fn set_status(&mut self, bit: CPUStatusBit, set: bool) {
        self.status.set(bit, set);
    }

    fn get_status(&self, bit: CPUStatusBit) -> bool {
        self.status.get(bit)
    }

    fn turn_on_status(&mut self, bit: CPUStatusBit) {
        self.status.turn_on(bit);
    }

    fn turn_off_status(&mut self, bit: CPUStatusBit) {
        self.status.turn_off(bit);
    }

    fn update_status_Z_N(&mut self, result: u8) {
        use self::CPUStatusBit::{N, Z};
        self.set_status(Z, result == 0);
        self.set_status(N, result & 0b1000_0000 != 0);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read((0x0100 as u16) + self.sp as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.write((0x0100 as u16) + self.sp as u16, data);
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
struct CPUStatus {
    bits: u8,
}

#[derive(Clone, Copy)]
enum CPUStatusBit {
    C,
    Z,
    I,
    D,
    B,
    U,
    V,
    N,
}

impl CPUStatusBit {
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

impl CPUStatus {
    fn new() -> CPUStatus {
        CPUStatus { bits: 0 }
    }

    fn reset(&mut self) {
        self.bits = 0;
    }

    fn set_from_bits(&mut self, bits: u8) {
        self.bits = bits;
    }

    fn get(&self, bit: CPUStatusBit) -> bool {
        self.bits & (1 << bit.bit_offset()) != 0
    }

    fn set(&mut self, bit: CPUStatusBit, set: bool) {
        if set {
            self.turn_on(bit);
        } else {
            self.turn_off(bit);
        }
    }

    fn turn_on(&mut self, bit: CPUStatusBit) {
        self.bits = self.bits | (1 << bit.bit_offset());
    }

    fn turn_off(&mut self, bit: CPUStatusBit) {
        self.bits = self.bits & !(1 << bit.bit_offset());
    }
}

#[derive(Clone, Copy)]
pub struct Instruction {
    opcode_byte: u8,
    oprand_addr: u16,
    spec: Spec,
    cycles: usize,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::Cartridge;

    fn new_cpu_with_program(program: Vec<u8>) -> CPU {
        let cart = Cartridge::new_from_program(program);
        let bus = Bus::new(cart);
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu
    }

    #[test]
    fn test_load_program() {
        let cart = Cartridge::new_from_program(vec![0x01, 0x23, 0x34, 0x00]);
        let bus = Bus::new(cart);
        let mut cpu = CPU::new(bus);
        assert_eq!(cpu.read(cpu.pc), 0x01);
        assert_eq!(cpu.read(cpu.pc + 1), 0x23);
        assert_eq!(cpu.read(cpu.pc + 2), 0x34);
        assert_eq!(cpu.read(cpu.pc + 3), 0x00);
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
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0xC000;
        assert_addr_eq(inst.oprand_addr, expected);

        // STA $0200,X
        let mut cpu = new_cpu_with_program(vec![0x9d, 0x00, 0x02]);
        cpu.reg_x = 0x01;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x0201;
        assert_addr_eq(inst.oprand_addr, expected);

        // STA $0200,Y
        let mut cpu = new_cpu_with_program(vec![0x99, 0x00, 0x02]);
        cpu.reg_y = 0x01;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x0201;
        assert_addr_eq(inst.oprand_addr, expected);

        // STA $c0
        let mut cpu = new_cpu_with_program(vec![0x85, 0xc0]);
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x00c0;
        assert_addr_eq(inst.oprand_addr, expected);

        // STA $c0,X
        let mut cpu = new_cpu_with_program(vec![0x95, 0xc0]);
        cpu.reg_x = 0x01;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x00c1;
        assert_addr_eq(inst.oprand_addr, expected);

        // LDX $c0,Y
        let mut cpu = new_cpu_with_program(vec![0xb6, 0xc0]);
        cpu.reg_y = 0x01;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x00c1;
        assert_addr_eq(inst.oprand_addr, expected);

        // LDX #$c0
        let mut cpu = new_cpu_with_program(vec![0xa2, 0xc0]);
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x8001;
        assert_addr_eq(inst.oprand_addr, expected);

        // BNE not_equal
        // not_equal: BRK
        let mut cpu = new_cpu_with_program(vec![0xd0, 0x00, 0x00]);
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x8002;
        assert_addr_eq(inst.oprand_addr, expected);

        // INX
        let mut cpu = new_cpu_with_program(vec![0xe8]);
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0;
        assert_addr_eq(inst.oprand_addr, expected);

        // JMP ($00f0)
        let mut cpu = new_cpu_with_program(vec![0x6c, 0xf0, 0x00]);
        cpu.write(0x00f0, 0x12);
        cpu.write(0x00f1, 0x34);
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x3412;
        assert_addr_eq(inst.oprand_addr, expected);

        // LDA ($c0,X)
        let mut cpu = new_cpu_with_program(vec![0xa1, 0xc0]);
        cpu.write(0x00c1, 0x12);
        cpu.write(0x00c2, 0x34);
        cpu.reg_x = 1;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x3412;
        assert_addr_eq(inst.oprand_addr, expected);

        // LDA ($c0),Y
        let mut cpu = new_cpu_with_program(vec![0xb1, 0xc0]);
        cpu.write(0x00c0, 0x12);
        cpu.write(0x00c1, 0x34);
        cpu.reg_y = 1;
        let inst = cpu.fetch_next_instruction();
        let expected: u16 = 0x3413;
        assert_addr_eq(inst.oprand_addr, expected);
    }

    #[test]
    fn test_cpu_status() {
        use super::CPUStatusBit::*;

        let mut status = CPUStatus::new();
        assert_eq!(status.bits, 0b0000_0000);

        status.set(C, true);
        status.turn_on(U);
        assert_eq!(status.bits, 0b0010_0001);

        status.turn_off(U);
        assert_eq!(status.bits, 0b0000_0001);
    }
}

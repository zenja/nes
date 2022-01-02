use super::Instruction;
use super::CPU;

impl CPU {
    pub fn trace(&mut self) -> String {
        let pc = self.pc;
        let inst = self.peak_next_instruction();
        let inst_bytes: Vec<u8> = match inst.spec.addr_mode.size() {
            0 => vec![inst.opcode_byte],
            1 => vec![inst.opcode_byte, self.read(pc + 1)],
            2 => vec![inst.opcode_byte, self.read(pc + 1), self.read(pc + 2)],
            _ => panic!("invalid addr mode size: {}", inst.spec.addr_mode.size()),
        };
        let inst_bytes_str: String = inst_bytes
            .into_iter()
            .map(|b| format!("{:02X?}", b))
            .collect::<Vec<String>>()
            .join(" ");
        let asm = CPU::disassemble(self, &inst);
        format!(
            "{:04X?}  {:8} {:31}  A:{:02X?} X:{:02X?} Y:{:02X?} P:{:02X?} SP:{:02X?} CYC:{}",
            pc,
            inst_bytes_str,
            asm,
            self.acc,
            self.reg_x,
            self.reg_y,
            self.status.bits,
            self.sp,
            self.total_cycles
        )
    }

    fn disassemble(&mut self, inst: &Instruction) -> String {
        use super::spec::Opcode::*;
        use super::AddrMode::*;

        let mut asm: String = format!(
            "{}{:?} ",
            if inst.spec.is_official { " " } else { "*" },
            inst.spec.opcode
        );

        let next_u8: u8 = self.read(self.pc + 1);
        let next_u16: u16 = self.read_u16(self.pc + 1);
        let oprands_asm: String = match inst.spec.addr_mode {
            Absolute => match inst.spec.opcode {
                JMP | JSR => format!("${:04X?}", inst.oprand_addr),
                _ => format!(
                    "${:04X?} = {:02X?}",
                    inst.oprand_addr,
                    self.read(inst.oprand_addr)
                ),
            },
            AbsoluteX => format!(
                "${:04X?},X @ {:04X?} = {:02X?}",
                next_u16,
                inst.oprand_addr,
                self.read(inst.oprand_addr)
            ),
            AbsoluteY => format!(
                "${:04X?},Y @ {:04X?} = {:02X?}",
                next_u16,
                inst.oprand_addr,
                self.read(inst.oprand_addr)
            ),
            ZeroPage => format!(
                "${:02X?} = {:02X?}",
                inst.oprand_addr,
                self.read(inst.oprand_addr)
            ),
            ZeroPageX => format!(
                "${:02X?},X @ {:02X?} = {:02X?}",
                next_u8,
                inst.oprand_addr as u8,
                self.read(inst.oprand_addr)
            ),
            ZeroPageY => format!(
                "${:02X?},Y @ {:02X?} = {:02X?}",
                next_u8,
                inst.oprand_addr as u8,
                self.read(inst.oprand_addr)
            ),
            Immediate => format!("#${:02X?}", self.read(inst.oprand_addr)),
            Relative => format!("${:04X}", inst.oprand_addr),
            Implicit => match inst.spec.opcode {
                ASL | LSR | ROL | ROR => "A".to_string(),
                _ => "".to_string(),
            },
            Indirect => {
                let addr_before_indirect = next_u16;
                let oprand_addr: u16 = if let JMP = inst.spec.opcode {
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
                format!("(${:04X?}) = {:04X?}", addr_before_indirect, oprand_addr)
            }
            IndexedIndirect => {
                format!(
                    "(${:02X?},X) @ {:02X?} = {:04X?} = {:02X?}",
                    next_u8,
                    next_u8.wrapping_add(self.reg_x),
                    inst.oprand_addr,
                    self.read(inst.oprand_addr)
                )
            }
            IndirectIndexed => {
                let addr_before_add_y: u16 = if next_u8 == 0xFF {
                    let a = self.read(0x00FF);
                    let b = self.read(0x0000);
                    u16::from_le_bytes([a, b])
                } else {
                    self.read_u16(next_u8 as u16)
                };
                format!(
                    "(${:02X?}),Y = {:04X?} @ {:04X?} = {:02X?}",
                    next_u8,
                    addr_before_add_y,
                    inst.oprand_addr,
                    self.read(inst.oprand_addr)
                )
            }
        };

        asm.push_str(&oprands_asm);
        asm
    }
}

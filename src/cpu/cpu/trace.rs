use super::Cpu;
use super::Instruction;

impl Cpu {
    pub fn trace(&mut self) -> String {
        let pc = self.pc;
        let inst = self.peak_next_instruction();
        let inst_bytes: Vec<u8> = match inst.spec.addr_mode.size() {
            0 => vec![inst.opcode_byte],
            1 => vec![inst.opcode_byte, self.bus.cpu_read(pc + 1)],
            2 => vec![
                inst.opcode_byte,
                self.bus.cpu_read(pc + 1),
                self.bus.cpu_read(pc + 2),
            ],
            _ => panic!("invalid addr mode size: {}", inst.spec.addr_mode.size()),
        };
        let inst_bytes_str: String = inst_bytes
            .into_iter()
            .map(|b| format!("{:02X?}", b))
            .collect::<Vec<String>>()
            .join(" ");
        let asm = Cpu::disassemble(self, &inst);
        format!(
            "{:04X?}  {:8}  {:30}  A:{:02X?} X:{:02X?} Y:{:02X?} P:{:02X?} SP:{:02X?} CYC:{}",
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

    fn disassemble(&self, inst: &Instruction) -> String {
        use super::AddrMode::*;

        let mut asm: String = format!("{:?} ", inst.spec.opcode);

        let next_u8: u8 = self.bus.cpu_read(self.pc + 1);
        let next_u16: u16 = self.read_u16(self.pc + 1);
        let oprands_asm: String = match inst.spec.addr_mode {
            Absolute => format!("${:04X?}", inst.oprand_addr),
            AbsoluteX => format!(
                "${:04X?},X @ {:04X?} = {:02X?}",
                next_u16,
                inst.oprand_addr,
                self.bus.cpu_read(inst.oprand_addr)
            ),
            AbsoluteY => format!(
                "${:04X?},Y @ {:04X?} = {:02X?}",
                next_u16,
                inst.oprand_addr,
                self.bus.cpu_read(inst.oprand_addr)
            ),
            ZeroPage => format!(
                "${:02X?} = {:02X?}",
                inst.oprand_addr,
                self.bus.cpu_read(inst.oprand_addr)
            ),
            ZeroPageX => format!(
                "${:02X?},X @ {:02X?} = {:02X?}",
                next_u8,
                inst.oprand_addr as u8,
                self.bus.cpu_read(inst.oprand_addr)
            ),
            ZeroPageY => format!(
                "${:02X?},Y @ {:02X?} = {:02X?}",
                next_u8,
                inst.oprand_addr as u8,
                self.bus.cpu_read(inst.oprand_addr)
            ),
            Immediate => format!("#${:02X?}", self.bus.cpu_read(inst.oprand_addr)),
            Relative => format!("${:04X}", inst.oprand_addr),
            Implicit => "".to_string(),
            Indirect => format!("(${:04X?}) = {:04X?}", next_u16, inst.oprand_addr),
            IndexedIndirect => {
                format!(
                    "(${:02X?},X) @ {:02X?} = {:04X?} = {:02X?}",
                    next_u8,
                    next_u8.wrapping_add(self.reg_x),
                    inst.oprand_addr,
                    self.bus.cpu_read(inst.oprand_addr)
                )
            }
            IndirectIndexed => {
                format!(
                    "(${:02X?}),Y = 0{:04X?} @ 0{:04X?} = {:02X?}",
                    next_u8,
                    self.bus.cpu_read(next_u8 as u16),
                    inst.oprand_addr,
                    self.bus.cpu_read(inst.oprand_addr)
                )
            }
        };

        asm.push_str(&oprands_asm);
        asm
    }
}

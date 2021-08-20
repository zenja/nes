use std::collections::HashMap;

use super::addr::*;

// (opcode byte, opcode, addr mode, base cycles, extra cycles cross page, is official)
const SPEC_TABLE: &'static [(u8, Opcode, AddrMode, u8, bool, bool)] = {
    use super::addr::AddrMode::*;
    use Opcode::*;
    &[
        // ADC
        (0x69, ADC, Immediate, 2, false, true),
        (0x65, ADC, ZeroPage, 3, false, true),
        (0x75, ADC, ZeroPageX, 4, false, true),
        (0x6D, ADC, Absolute, 4, false, true),
        (0x7D, ADC, AbsoluteX, 4, true, true),
        (0x79, ADC, AbsoluteY, 4, true, true),
        (0x61, ADC, IndexedIndirect, 6, false, true),
        (0x71, ADC, IndirectIndexed, 5, true, true),
        // AND
        (0x29, AND, Immediate, 2, false, true),
        (0x25, AND, ZeroPage, 3, false, true),
        (0x35, AND, ZeroPageX, 4, false, true),
        (0x2D, AND, Absolute, 4, false, true),
        (0x3D, AND, AbsoluteX, 4, true, true),
        (0x39, AND, AbsoluteY, 4, true, true),
        (0x21, AND, IndexedIndirect, 6, false, true),
        (0x31, AND, IndirectIndexed, 5, true, true),
        // ASL
        (0x0A, ASL, Implicit, 2, false, true),
        (0x06, ASL, ZeroPage, 5, false, true),
        (0x16, ASL, ZeroPageX, 6, false, true),
        (0x0E, ASL, Absolute, 6, false, true),
        (0x1E, ASL, AbsoluteX, 7, false, true),
        // BCC
        (0x90, BCC, Relative, 2, true, true),
        // BCS
        (0xB0, BCS, Relative, 2, true, true),
        // BEQ
        (0xF0, BEQ, Relative, 2, true, true),
        // BIT
        (0x24, BIT, ZeroPage, 3, false, true),
        (0x2C, BIT, Absolute, 4, false, true),
        // BMI
        (0x30, BMI, Relative, 2, true, true),
        // BNE
        (0xD0, BNE, Relative, 2, true, true),
        // BPL
        (0x10, BPL, Relative, 2, true, true),
        // BRK
        (0x00, BRK, Implicit, 7, false, true),
        // BVC
        (0x50, BVC, Relative, 2, true, true),
        // BVS
        (0x70, BVS, Relative, 2, true, true),
        // CLC
        (0x18, CLC, Implicit, 2, false, true),
        // CLD
        (0xD8, CLD, Implicit, 2, false, true),
        // CLI
        (0x58, CLI, Implicit, 2, false, true),
        // CLV
        (0xB8, CLV, Implicit, 2, false, true),
        // CMP
        (0xC9, CMP, Immediate, 2, false, true),
        (0xC5, CMP, ZeroPage, 3, false, true),
        (0xD5, CMP, ZeroPageX, 4, false, true),
        (0xCD, CMP, Absolute, 4, false, true),
        (0xDD, CMP, AbsoluteX, 4, true, true),
        (0xD9, CMP, AbsoluteY, 4, true, true),
        (0xC1, CMP, IndexedIndirect, 6, false, true),
        (0xD1, CMP, IndirectIndexed, 5, true, true),
        // CPX
        (0xE0, CPX, Immediate, 2, false, true),
        (0xE4, CPX, ZeroPage, 3, false, true),
        (0xEC, CPX, Absolute, 4, false, true),
        // CPY
        (0xC0, CPY, Immediate, 2, false, true),
        (0xC4, CPY, ZeroPage, 3, false, true),
        (0xCC, CPY, Absolute, 4, false, true),
        // DEC
        (0xC6, DEC, ZeroPage, 5, false, true),
        (0xD6, DEC, ZeroPageX, 6, false, true),
        (0xCE, DEC, Absolute, 6, false, true),
        (0xDE, DEC, AbsoluteX, 7, false, true),
        // DEX
        (0xCA, DEX, Implicit, 2, false, true),
        // DEY
        (0x88, DEY, Implicit, 2, false, true),
        // EOR
        (0x49, EOR, Immediate, 2, false, true),
        (0x45, EOR, ZeroPage, 3, false, true),
        (0x55, EOR, ZeroPageX, 4, false, true),
        (0x4D, EOR, Absolute, 4, false, true),
        (0x5D, EOR, AbsoluteX, 4, true, true),
        (0x59, EOR, AbsoluteY, 4, true, true),
        (0x41, EOR, IndexedIndirect, 6, false, true),
        (0x51, EOR, IndirectIndexed, 5, true, true),
        // INC
        (0xE6, INC, ZeroPage, 5, false, true),
        (0xF6, INC, ZeroPageX, 6, false, true),
        (0xEE, INC, Absolute, 6, false, true),
        (0xFE, INC, AbsoluteX, 7, false, true),
        // INX
        (0xE8, INX, Implicit, 2, false, true),
        // INY
        (0xC8, INY, Implicit, 2, false, true),
        // JMP
        (0x4C, JMP, Absolute, 3, false, true),
        (0x6C, JMP, Indirect, 5, false, true),
        // JSR
        (0x20, JSR, Absolute, 6, false, true),
        // LDA
        (0xA9, LDA, Immediate, 2, false, true),
        (0xA5, LDA, ZeroPage, 3, false, true),
        (0xB5, LDA, ZeroPageX, 4, false, true),
        (0xAD, LDA, Absolute, 4, false, true),
        (0xBD, LDA, AbsoluteX, 4, true, true),
        (0xB9, LDA, AbsoluteY, 4, true, true),
        (0xA1, LDA, IndexedIndirect, 6, false, true),
        (0xB1, LDA, IndirectIndexed, 5, true, true),
        // LDX
        (0xA2, LDX, Immediate, 2, false, true),
        (0xA6, LDX, ZeroPage, 3, false, true),
        (0xB6, LDX, ZeroPageY, 4, false, true),
        (0xAE, LDX, Absolute, 4, false, true),
        (0xBE, LDX, AbsoluteY, 4, true, true),
        // LDY
        (0xA0, LDY, Immediate, 2, false, true),
        (0xA4, LDY, ZeroPage, 3, false, true),
        (0xB4, LDY, ZeroPageX, 4, false, true),
        (0xAC, LDY, Absolute, 4, false, true),
        (0xBC, LDY, AbsoluteX, 4, true, true),
        // LSR
        (0x4A, LSR, Implicit, 2, false, true),
        (0x46, LSR, ZeroPage, 5, false, true),
        (0x56, LSR, ZeroPageX, 6, false, true),
        (0x4E, LSR, Absolute, 6, false, true),
        (0x5E, LSR, AbsoluteX, 7, false, true),
        // NOP
        (0xEA, NOP, Implicit, 2, false, true),
        // ORA
        (0x09, ORA, Immediate, 2, false, true),
        (0x05, ORA, ZeroPage, 3, false, true),
        (0x15, ORA, ZeroPageX, 4, false, true),
        (0x0D, ORA, Absolute, 4, false, true),
        (0x1D, ORA, AbsoluteX, 4, true, true),
        (0x19, ORA, AbsoluteY, 4, true, true),
        (0x01, ORA, IndexedIndirect, 6, false, true),
        (0x11, ORA, IndirectIndexed, 5, true, true),
        // PHA
        (0x48, PHA, Implicit, 3, false, true),
        // PHP
        (0x08, PHP, Implicit, 3, false, true),
        // PLA
        (0x68, PLA, Implicit, 4, false, true),
        // PLP
        (0x28, PLP, Implicit, 4, false, true),
        // ROL
        (0x2A, ROL, Implicit, 2, false, true),
        (0x26, ROL, ZeroPage, 5, false, true),
        (0x36, ROL, ZeroPageX, 6, false, true),
        (0x2E, ROL, Absolute, 6, false, true),
        (0x3E, ROL, AbsoluteX, 7, false, true),
        // ROR
        (0x6A, ROR, Implicit, 2, false, true),
        (0x66, ROR, ZeroPage, 5, false, true),
        (0x76, ROR, ZeroPageX, 6, false, true),
        (0x6E, ROR, Absolute, 6, false, true),
        (0x7E, ROR, AbsoluteX, 7, false, true),
        // RTI
        (0x40, RTI, Implicit, 6, false, true),
        // RTS
        (0x60, RTS, Implicit, 6, false, true),
        // SBC
        (0xE9, SBC, Immediate, 2, false, true),
        (0xE5, SBC, ZeroPage, 3, false, true),
        (0xF5, SBC, ZeroPageX, 4, false, true),
        (0xED, SBC, Absolute, 4, false, true),
        (0xFD, SBC, AbsoluteX, 4, true, true),
        (0xF9, SBC, AbsoluteY, 4, true, true),
        (0xE1, SBC, IndexedIndirect, 6, false, true),
        (0xF1, SBC, IndirectIndexed, 5, true, true),
        // SEC
        (0x38, SEC, Implicit, 2, false, true),
        // SED
        (0xF8, SED, Implicit, 2, false, true),
        // SEI
        (0x78, SEI, Implicit, 2, false, true),
        // STA
        (0x85, STA, ZeroPage, 3, false, true),
        (0x95, STA, ZeroPageX, 4, false, true),
        (0x8D, STA, Absolute, 4, false, true),
        (0x9D, STA, AbsoluteX, 5, false, true),
        (0x99, STA, AbsoluteY, 5, false, true),
        (0x81, STA, IndexedIndirect, 6, false, true),
        (0x91, STA, IndirectIndexed, 6, false, true),
        // STX
        (0x86, STX, ZeroPage, 3, false, true),
        (0x96, STX, ZeroPageY, 4, false, true),
        (0x8E, STX, Absolute, 4, false, true),
        // STY
        (0x84, STY, ZeroPage, 3, false, true),
        (0x94, STY, ZeroPageX, 4, false, true),
        (0x8C, STY, Absolute, 4, false, true),
        // TAX
        (0xAA, TAX, Implicit, 2, false, true),
        // TAY
        (0xA8, TAY, Implicit, 2, false, true),
        // TSX
        (0xBA, TSX, Implicit, 2, false, true),
        // TXA
        (0x8A, TXA, Implicit, 2, false, true),
        // TXS
        (0x9A, TXS, Implicit, 2, false, true),
        // TYA
        (0x98, TYA, Implicit, 2, false, true),
        // ---------- Unofficial Opcodes ----------
        // Ref: https://www.nesdev.com/undocumented_opcodes.txt
        // NOP
        (0x1A, NOP, Implicit, 2, false, false),
        (0x3A, NOP, Implicit, 2, false, false),
        (0x5A, NOP, Implicit, 2, false, false),
        (0x7A, NOP, Implicit, 2, false, false),
        (0xDA, NOP, Implicit, 2, false, false),
        (0xFA, NOP, Implicit, 2, false, false),
        // NOP (DOP)
        (0x04, NOP, ZeroPage, 3, false, false),
        (0x14, NOP, ZeroPageX, 4, false, false),
        (0x34, NOP, ZeroPageX, 4, false, false),
        (0x44, NOP, ZeroPage, 3, false, false),
        (0x54, NOP, ZeroPageX, 4, false, false),
        (0x64, NOP, ZeroPage, 3, false, false),
        (0x74, NOP, ZeroPageX, 4, false, false),
        (0x80, NOP, Immediate, 2, false, false),
        (0x82, NOP, Immediate, 2, false, false),
        (0x89, NOP, Immediate, 2, false, false),
        (0xC2, NOP, Immediate, 2, false, false),
        (0xD4, NOP, ZeroPageX, 4, false, false),
        (0xE2, NOP, Immediate, 2, false, false),
        (0xF4, NOP, ZeroPageX, 4, false, false),
        // NOP (TOP)
        (0x0C, NOP, Absolute, 4, false, false),
        (0x1C, NOP, AbsoluteX, 4, true, false),
        (0x3C, NOP, AbsoluteX, 4, true, false),
        (0x5C, NOP, AbsoluteX, 4, true, false),
        (0x7C, NOP, AbsoluteX, 4, true, false),
        (0xDC, NOP, AbsoluteX, 4, true, false),
        (0xFC, NOP, AbsoluteX, 4, true, false),
    ]
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Opcode {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
}

#[derive(Clone, Copy)]
pub struct Spec {
    pub opcode_byte: u8,
    pub opcode: Opcode,
    pub addr_mode: AddrMode,
    pub base_cycles: u8,
    pub inc_cycle_on_page_crossed: bool,
    pub is_official: bool,
}

pub fn opcode_to_spec() -> HashMap<u8, Spec> {
    let mut map: HashMap<u8, Spec> = HashMap::with_capacity(SPEC_TABLE.len());
    for (opcode_byte, opcode, addr_mode, base_cycles, inc_cycle_on_page_crossed, is_official) in
        SPEC_TABLE
    {
        map.insert(
            *opcode_byte,
            Spec {
                opcode_byte: *opcode_byte,
                opcode: *opcode,
                addr_mode: *addr_mode,
                base_cycles: *base_cycles,
                inc_cycle_on_page_crossed: *inc_cycle_on_page_crossed,
                is_official: *is_official,
            },
        );
    }
    map
}

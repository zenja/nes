use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

pub fn assemble(asm: &str) -> Vec<u8> {
    assemble_with_start_addr(asm, 0x0600)
}

pub fn assemble_with_start_addr(asm: &str, start_addr: u16) -> Vec<u8> {
    let lines = asm.split("\n").into_iter().map(|x| x.to_string()).collect();
    let assembler = Assembler::new(lines);
    assembler.assemble(start_addr)
}

#[allow(dead_code)]
struct Assembler {
    lines: Vec<String>,
    params: HashMap<String, String>,
    label_to_addr: HashMap<String, u16>,
}

impl Assembler {
    fn new(lines: Vec<String>) -> Self {
        Assembler {
            lines: lines,
            params: HashMap::new(),
            label_to_addr: HashMap::new(),
        }
    }

    fn pre_process(&mut self) {
        // remove comments, trim, and to upper case
        for l in self.lines.iter_mut() {
            if let Some(i) = l.find(';') {
                *l = l[..i].to_string();
            }
            *l = l.trim().to_uppercase().to_string();
        }
        // remove empty lines
        self.lines.retain(|l| !l.trim().is_empty());
    }

    fn assemble(mut self, start_addr: u16) -> Vec<u8> {
        use Statement::*;

        self.pre_process();

        // replace defined params
        for l in self.lines.iter_mut() {
            if let Some(Define { name, value }) = parse_statement(&l) {
                self.params.insert(name.to_string(), value.to_string());
            } else {
                for (p, v) in &self.params {
                    *l = l.replace(p, v);
                }
            }
        }

        // parse to statements after params replacement
        let mut statements: Vec<Statement> = self
            .lines
            .iter()
            .map(|l| match parse_statement(&l) {
                Some(s) => s,
                None => panic!("failed to parse code '{}'", l),
            })
            .collect();

        // calculate addr for labels
        let mut curr_addr = start_addr;
        for s in statements.iter() {
            match s {
                Label { name } => {
                    self.label_to_addr.insert(name.to_uppercase(), curr_addr);
                }
                Instruction { opcode, addr_mode } => {
                    curr_addr += instruction_size(&opcode, &addr_mode) as u16;
                }
                _ => {}
            }
        }

        // replace relative label to relative addr or absolute addr
        let mut curr_addr = start_addr;
        for s in statements.iter_mut() {
            if let Instruction { opcode, addr_mode } = s {
                curr_addr += instruction_size(&opcode, &addr_mode) as u16;
                if let AddrMode::RelativeLabel(label) = addr_mode {
                    let label_addr: u16 = *self.label_to_addr.get(&label.to_uppercase()).unwrap();
                    *s = Instruction {
                        opcode: opcode.to_string(),
                        addr_mode: label_to_relative_or_absolute(opcode, curr_addr, label_addr),
                    }
                }
            }
        }

        // assemble each instruction
        let mut result: Vec<u8> = vec![];
        for s in statements.iter() {
            result.extend(s.assemble());
        }
        result
    }
}

fn label_to_relative_or_absolute(opcode: &str, curr_addr: u16, label_addr: u16) -> AddrMode {
    let relative_opcodes: Vec<&str> = vec!["BCC", "BCS", "BEQ", "BMI", "BNE", "BPL", "BVC", "BVS"];
    if relative_opcodes.contains(&opcode) {
        let relative_addr: i8 = (label_addr as i32 - curr_addr as i32) as i8;
        AddrMode::Relative(relative_addr)
    } else {
        AddrMode::Absolute(label_addr)
    }
}

#[derive(Debug, PartialEq)]
enum Statement {
    Define { name: String, value: String },
    Label { name: String },
    Instruction { opcode: String, addr_mode: AddrMode },
}

impl Statement {
    fn assemble(&self) -> Vec<u8> {
        use AddrMode::*;

        fn panic_addr_mode_not_supported(opcode: &str, addr_mode: &AddrMode) -> ! {
            panic!("{} does not support addr mode {:?}", opcode, addr_mode)
        }

        match &self {
            Statement::Define { .. } => vec![],
            Statement::Label { .. } => vec![],
            Statement::Instruction { opcode, addr_mode } => {
                // Ref: http://www.obelisk.me.uk/6502/reference.html
                let asm_opcode: u8 = match &opcode.to_uppercase()[..] {
                    opcode @ "ADC" => match addr_mode {
                        Immediate(_) => 0x69,
                        ZeroPage(_) => 0x65,
                        ZeroPageX(_) => 0x75,
                        Absolute(_) => 0x6D,
                        AbsoluteX(_) => 0x7D,
                        AbsoluteY(_) => 0x79,
                        IndexedIndirect(_) => 0x61,
                        IndirectIndexed(_) => 0x71,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "AND" => match addr_mode {
                        Immediate(_) => 0x29,
                        ZeroPage(_) => 0x25,
                        ZeroPageX(_) => 0x35,
                        Absolute(_) => 0x2D,
                        AbsoluteX(_) => 0x3D,
                        AbsoluteY(_) => 0x39,
                        IndexedIndirect(_) => 0x21,
                        IndirectIndexed(_) => 0x31,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "ASL" => match addr_mode {
                        Implicit => 0x0A,
                        ZeroPage(_) => 0x06,
                        ZeroPageX(_) => 0x16,
                        Absolute(_) => 0x0E,
                        AbsoluteX(_) => 0x1E,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BCC" => match addr_mode {
                        Relative(_) => 0x90,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BCS" => match addr_mode {
                        Relative(_) => 0xB0,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BEQ" => match addr_mode {
                        Relative(_) => 0xF0,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BIT" => match addr_mode {
                        ZeroPage(_) => 0x24,
                        Absolute(_) => 0x2C,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BMI" => match addr_mode {
                        Relative(_) => 0x30,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BNE" => match addr_mode {
                        Relative(_) => 0xD0,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BPL" => match addr_mode {
                        Relative(_) => 0x10,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BRK" => match addr_mode {
                        Implicit => 0x00,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BVC" => match addr_mode {
                        Relative(_) => 0x50,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "BVS" => match addr_mode {
                        Relative(_) => 0x70,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CLC" => match addr_mode {
                        Implicit => 0x18,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CLD" => match addr_mode {
                        Implicit => 0xD8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CLI" => match addr_mode {
                        Implicit => 0x58,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CLV" => match addr_mode {
                        Implicit => 0xB8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CMP" => match addr_mode {
                        Immediate(_) => 0xC9,
                        ZeroPage(_) => 0xC5,
                        ZeroPageX(_) => 0xD5,
                        Absolute(_) => 0xCD,
                        AbsoluteX(_) => 0xDD,
                        AbsoluteY(_) => 0xD9,
                        IndexedIndirect(_) => 0xC1,
                        IndirectIndexed(_) => 0xD1,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CPX" => match addr_mode {
                        Immediate(_) => 0xE0,
                        ZeroPage(_) => 0xE4,
                        Absolute(_) => 0xEC,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "CPY" => match addr_mode {
                        Immediate(_) => 0xC0,
                        ZeroPage(_) => 0xC4,
                        Absolute(_) => 0xCC,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "DEC" => match addr_mode {
                        ZeroPage(_) => 0xC6,
                        ZeroPageX(_) => 0xD6,
                        Absolute(_) => 0xCE,
                        AbsoluteX(_) => 0xDE,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "DEX" => match addr_mode {
                        Implicit => 0xCA,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "DEY" => match addr_mode {
                        Implicit => 0x88,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "EOR" => match addr_mode {
                        Immediate(_) => 0x49,
                        ZeroPage(_) => 0x45,
                        ZeroPageX(_) => 0x55,
                        Absolute(_) => 0x4D,
                        AbsoluteX(_) => 0x5D,
                        AbsoluteY(_) => 0x59,
                        IndexedIndirect(_) => 0x41,
                        IndirectIndexed(_) => 0x51,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "INC" => match addr_mode {
                        ZeroPage(_) => 0xE6,
                        ZeroPageX(_) => 0xF6,
                        Absolute(_) => 0xEE,
                        AbsoluteX(_) => 0xFE,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "INX" => match addr_mode {
                        Implicit => 0xE8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "INY" => match addr_mode {
                        Implicit => 0xC8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "JMP" => match addr_mode {
                        Absolute(_) => 0x4C,
                        Indirect(_) => 0x6C,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "JSR" => match addr_mode {
                        Absolute(_) => 0x20,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "LDA" => match addr_mode {
                        Immediate(_) => 0xA9,
                        ZeroPage(_) => 0xA5,
                        ZeroPageX(_) => 0xB5,
                        Absolute(_) => 0xAD,
                        AbsoluteX(_) => 0xBD,
                        AbsoluteY(_) => 0xB9,
                        IndexedIndirect(_) => 0xA1,
                        IndirectIndexed(_) => 0xB1,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "LDX" => match addr_mode {
                        Immediate(_) => 0xA2,
                        ZeroPage(_) => 0xA6,
                        ZeroPageY(_) => 0xB6,
                        Absolute(_) => 0xAE,
                        AbsoluteY(_) => 0xBE,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "LDY" => match addr_mode {
                        Immediate(_) => 0xA0,
                        ZeroPage(_) => 0xA4,
                        ZeroPageY(_) => 0xB4,
                        Absolute(_) => 0xAC,
                        AbsoluteY(_) => 0xBC,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "LSR" => match addr_mode {
                        Implicit => 0x4A,
                        ZeroPage(_) => 0x46,
                        ZeroPageX(_) => 0x56,
                        Absolute(_) => 0x4E,
                        AbsoluteX(_) => 0x5E,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "NOP" => match addr_mode {
                        Implicit => 0xEA,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "ORA" => match addr_mode {
                        Immediate(_) => 0x09,
                        ZeroPage(_) => 0x05,
                        ZeroPageX(_) => 0x15,
                        Absolute(_) => 0x0D,
                        AbsoluteX(_) => 0x1D,
                        AbsoluteY(_) => 0x19,
                        IndexedIndirect(_) => 0x01,
                        IndirectIndexed(_) => 0x11,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "PHA" => match addr_mode {
                        Implicit => 0x48,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "PHP" => match addr_mode {
                        Implicit => 0x08,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "PLA" => match addr_mode {
                        Implicit => 0x68,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "PLP" => match addr_mode {
                        Implicit => 0x28,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "ROL" => match addr_mode {
                        Implicit => 0x2A,
                        ZeroPage(_) => 0x26,
                        ZeroPageX(_) => 0x36,
                        Absolute(_) => 0x2E,
                        AbsoluteX(_) => 0x3E,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "ROR" => match addr_mode {
                        Implicit => 0x6A,
                        ZeroPage(_) => 0x66,
                        ZeroPageX(_) => 0x76,
                        Absolute(_) => 0x6E,
                        AbsoluteX(_) => 0x7E,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "RTI" => match addr_mode {
                        Implicit => 0x40,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "RTS" => match addr_mode {
                        Implicit => 0x60,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "SBC" => match addr_mode {
                        Immediate(_) => 0xE9,
                        ZeroPage(_) => 0xE5,
                        ZeroPageX(_) => 0xF5,
                        Absolute(_) => 0xED,
                        AbsoluteX(_) => 0xFD,
                        AbsoluteY(_) => 0xF9,
                        IndexedIndirect(_) => 0xE1,
                        IndirectIndexed(_) => 0xF1,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "SEC" => match addr_mode {
                        Implicit => 0x38,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "SED" => match addr_mode {
                        Implicit => 0xF8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "SEI" => match addr_mode {
                        Implicit => 0x78,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "STA" => match addr_mode {
                        ZeroPage(_) => 0x85,
                        ZeroPageX(_) => 0x95,
                        Absolute(_) => 0x8D,
                        AbsoluteX(_) => 0x9D,
                        AbsoluteY(_) => 0x99,
                        IndexedIndirect(_) => 0x81,
                        IndirectIndexed(_) => 0x91,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "STX" => match addr_mode {
                        ZeroPage(_) => 0x86,
                        ZeroPageY(_) => 0x96,
                        Absolute(_) => 0x8E,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "STY" => match addr_mode {
                        ZeroPage(_) => 0x84,
                        ZeroPageY(_) => 0x94,
                        Absolute(_) => 0x8C,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TAX" => match addr_mode {
                        Implicit => 0xAA,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TAY" => match addr_mode {
                        Implicit => 0xA8,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TSX" => match addr_mode {
                        Implicit => 0xBA,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TXA" => match addr_mode {
                        Implicit => 0x8A,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TXS" => match addr_mode {
                        Implicit => 0x9A,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ "TYA" => match addr_mode {
                        Implicit => 0x98,
                        _ => panic_addr_mode_not_supported(opcode, addr_mode),
                    },
                    opcode @ _ => panic!("opcode unrecognized: {}", opcode),
                };
                let mut asm: Vec<u8> = vec![asm_opcode];
                asm.extend(&addr_mode.assemble());
                asm
            }
        }
    }
}

fn instruction_size(opcode: &str, addr_mode: &AddrMode) -> u8 {
    match addr_mode {
        AddrMode::Absolute(_) => 3,
        AddrMode::AbsoluteX(_) => 3,
        AddrMode::AbsoluteY(_) => 3,
        AddrMode::ZeroPage(_) => 2,
        AddrMode::ZeroPageX(_) => 2,
        AddrMode::ZeroPageY(_) => 2,
        AddrMode::Immediate(_) => 2,
        AddrMode::Relative(_) => 2,
        AddrMode::RelativeLabel(_) => {
            let relative_opcodes: Vec<&str> =
                vec!["BCC", "BCS", "BEQ", "BMI", "BNE", "BPL", "BVC", "BVS"];
            if relative_opcodes
                .iter()
                .any(|&op| opcode.to_uppercase() == op)
            {
                2
            } else {
                3
            }
        }
        AddrMode::Implicit => 1,
        AddrMode::Indirect(_) => 3,
        AddrMode::IndexedIndirect(_) => 2,
        AddrMode::IndirectIndexed(_) => 2,
    }
}

fn parse_statement(s: &str) -> Option<Statement> {
    lazy_static! {
        static ref DEFINE_RE: Regex = Regex::new(r"(?i)^define +([^ ]+) +([^ ]+)").unwrap();
        static ref LABEL_RE: Regex = Regex::new(r"(?i)^([^ :]+):$").unwrap();
        static ref INSTRUCTION_RE: Regex = Regex::new(r"(?i)^([a-z]{3}) *([^ ]*)$").unwrap();
    }
    if let Some(cap) = DEFINE_RE.captures_iter(s).next() {
        Some(Statement::Define {
            name: String::from(&cap[1]),
            value: String::from(&cap[2]),
        })
    } else if let Some(cap) = LABEL_RE.captures_iter(s).next() {
        Some(Statement::Label {
            name: String::from(&cap[1]),
        })
    } else if let Some(cap) = INSTRUCTION_RE.captures_iter(s).next() {
        let opcode = String::from(&cap[1]);
        match parse_addr_mode(&cap[2]) {
            Some(mode) => Some(Statement::Instruction {
                opcode: opcode,
                addr_mode: mode,
            }),
            None => None,
        }
    } else {
        None
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum AddrMode {
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Immediate(u8),
    Relative(i8),
    RelativeLabel(String),
    Implicit,
    Indirect(u16),
    IndexedIndirect(u8),
    IndirectIndexed(u8),
}

impl AddrMode {
    fn assemble(&self) -> Vec<u8> {
        fn to_little_endian_vec(a: u16) -> Vec<u8> {
            a.to_le_bytes().to_vec()
        }

        match self {
            AddrMode::Absolute(a) => to_little_endian_vec(*a),
            AddrMode::AbsoluteX(a) => to_little_endian_vec(*a),
            AddrMode::AbsoluteY(a) => to_little_endian_vec(*a),
            AddrMode::ZeroPage(a) => vec![*a],
            AddrMode::ZeroPageX(a) => vec![*a],
            AddrMode::ZeroPageY(a) => vec![*a],
            AddrMode::Immediate(a) => vec![*a],
            AddrMode::Relative(a) => vec![*a as u8],
            AddrMode::RelativeLabel(_) => panic!("cannot assemble relative mode with label"),
            AddrMode::Implicit => Vec::new(),
            AddrMode::Indirect(a) => to_little_endian_vec(*a),
            AddrMode::IndexedIndirect(a) => vec![*a],
            AddrMode::IndirectIndexed(a) => vec![*a],
        }
    }
}

fn parse_addr_mode(s: &str) -> Option<AddrMode> {
    use AddrMode::*;

    lazy_static! {
        static ref ABSOLUTE_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{4})$").unwrap();
        static ref ABSOLUTE_X_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{4}), *x$").unwrap();
        static ref ABSOLUTE_Y_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{4}), *y$").unwrap();
        static ref ZERO_PAGE_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{2})$").unwrap();
        static ref ZERO_PAGE_X_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{2}), *x$").unwrap();
        static ref ZERO_PAGE_Y_RE: Regex = Regex::new(r"(?i)^\$([0-9a-f]{2}), *y$").unwrap();
        static ref IMMEDIATE_HEX_RE: Regex = Regex::new(r"(?i)^#\$([0-9a-f]{1,2})$").unwrap();
        static ref IMMEDIATE_DEC_RE: Regex = Regex::new(r"(?i)^#([0-9a-f]{1,2})$").unwrap();
        static ref RELATIVE_RE: Regex = Regex::new(r"(?i)^\*([+-][0-9]{1,3})$").unwrap();
        static ref RELATIVE_LABEL_RE: Regex = Regex::new(r"(?i)^([a-z_]+)$").unwrap();
        static ref IMPLICIT_RE: Regex = Regex::new(r"(?i)^$").unwrap();
        static ref INDIRECT_RE: Regex = Regex::new(r"(?i)^\(\$([0-9a-f]{4})\)$").unwrap();
        static ref INDEXED_INDIRECT_RE: Regex =
            Regex::new(r"(?i)^\(\$([0-9a-f]{2}), *x\)$").unwrap();
        static ref INDIRECT_INDEXED_RE: Regex =
            Regex::new(r"(?i)^\(\$([0-9a-f]{2})\), *y$").unwrap();
    }
    if let Some(cap) = ABSOLUTE_RE.captures_iter(s).next() {
        Some(Absolute(u16::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = ABSOLUTE_X_RE.captures_iter(s).next() {
        Some(AbsoluteX(u16::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = ABSOLUTE_Y_RE.captures_iter(s).next() {
        Some(AbsoluteY(u16::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = ZERO_PAGE_RE.captures_iter(s).next() {
        Some(ZeroPage(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = ZERO_PAGE_X_RE.captures_iter(s).next() {
        Some(ZeroPageX(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = ZERO_PAGE_Y_RE.captures_iter(s).next() {
        Some(ZeroPageY(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = IMMEDIATE_HEX_RE.captures_iter(s).next() {
        Some(Immediate(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = IMMEDIATE_DEC_RE.captures_iter(s).next() {
        Some(Immediate(i8::from_str_radix(&cap[1], 16).unwrap() as u8))
    } else if let Some(cap) = RELATIVE_RE.captures_iter(s).next() {
        Some(Relative(i8::from_str_radix(&cap[1], 10).unwrap()))
    } else if let Some(cap) = RELATIVE_LABEL_RE.captures_iter(s).next() {
        Some(RelativeLabel(String::from(&cap[1])))
    } else if IMPLICIT_RE.is_match(s) {
        Some(Implicit)
    } else if let Some(cap) = INDIRECT_RE.captures_iter(s).next() {
        Some(Indirect(u16::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = INDEXED_INDIRECT_RE.captures_iter(s).next() {
        Some(IndexedIndirect(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else if let Some(cap) = INDIRECT_INDEXED_RE.captures_iter(s).next() {
        Some(IndirectIndexed(u8::from_str_radix(&cap[1], 16).unwrap()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_process() {
        let mut assembler = Assembler::new(vec![
            "  ldy #$01".to_string(),
            "  ;;; a comment".to_string(),
            "  Lda #$03 ; a comment".to_string(),
        ]);
        assembler.pre_process();
        assert_eq!(
            assembler.lines,
            vec!["LDY #$01".to_string(), "LDA #$03".to_string()]
        );
    }

    #[test]
    fn test_parse_define_statement() {
        let s = "DEFINE APPLEL         $00";
        let expected = Some(Statement::Define {
            name: "APPLEL".to_string(),
            value: "$00".to_string(),
        });
        assert_eq!(parse_statement(s), expected);
    }

    #[test]
    fn test_parse_label_statement() {
        let s = "LOOP:";
        let expected = Some(Statement::Label {
            name: "LOOP".to_string(),
        });
        assert_eq!(parse_statement(s), expected);
    }

    #[test]
    fn test_parse_instruction_statement() {
        use AddrMode::*;
        use Statement::Instruction;

        let codes = vec![
            "LDY #$01",
            "STA $01",
            "JMP ($00f0)",
            "LDA ($01),Y",
            "STX $0704",
            "BRK",
        ];
        let statements = vec![
            Instruction {
                opcode: "LDY".to_string(),
                addr_mode: Immediate(0x01),
            },
            Instruction {
                opcode: "STA".to_string(),
                addr_mode: ZeroPage(0x01),
            },
            Instruction {
                opcode: "JMP".to_string(),
                addr_mode: Indirect(0x00f0),
            },
            Instruction {
                opcode: "LDA".to_string(),
                addr_mode: IndirectIndexed(0x01),
            },
            Instruction {
                opcode: "STX".to_string(),
                addr_mode: Absolute(0x0704),
            },
            Instruction {
                opcode: "BRK".to_string(),
                addr_mode: Implicit,
            },
        ];
        for (c, s) in codes.iter().zip(statements.into_iter()) {
            assert_eq!(parse_statement(c), Some(s));
        }
    }

    #[test]
    fn test_parse_addr_mode() {
        use AddrMode::*;
        let addrs = vec![
            "$c000", "$c000, X", "$c000, Y", "$c0", "$c0,X", "$c0,Y", "#$c0", "*+4", "LOOP", "",
            "($c000)", "($c0, X)", "($c0),Y",
        ];
        let modes = vec![
            Absolute(0xc000),
            AbsoluteX(0xc000),
            AbsoluteY(0xc000),
            ZeroPage(0xc0),
            ZeroPageX(0xc0),
            ZeroPageY(0xc0),
            Immediate(0xc0),
            Relative(4i8),
            RelativeLabel("LOOP".to_string()),
            Implicit,
            Indirect(0xc000),
            IndexedIndirect(0xc0),
            IndirectIndexed(0xc0),
        ];
        for (addr, mode) in addrs.iter().zip(modes.into_iter()) {
            assert_eq!(parse_addr_mode(addr).unwrap(), mode);
        }
    }

    #[test]
    fn test_assemble_addr_mode() {
        use AddrMode::*;

        let modes: Vec<AddrMode> = vec![
            Absolute(0xc000),
            AbsoluteX(0xc000),
            AbsoluteY(0xc000),
            ZeroPage(0xc0),
            ZeroPageX(0xc0),
            ZeroPageY(0xc0),
            Immediate(0xc0),
            Relative(4i8),
            Implicit,
            Indirect(0xc000),
            IndexedIndirect(0xc0),
            IndirectIndexed(0xc0),
        ];
        let bytes: Vec<Vec<u8>> = vec![
            vec![0x00, 0xc0],
            vec![0x00, 0xc0],
            vec![0x00, 0xc0],
            vec![0xc0],
            vec![0xc0],
            vec![0xc0],
            vec![0xc0],
            vec![4u8],
            vec![],
            vec![0x00, 0xc0],
            vec![0xc0],
            vec![0xc0],
        ];
        for (m, b) in modes.iter().zip(bytes.into_iter()) {
            assert_eq!(m.assemble(), b);
        }
    }

    #[test]
    fn test_assemble_statement() {
        use itertools::izip;

        let codes = vec![
            "LDY #$01",
            "LDA #$03",
            "STA $01",
            "LDA #$07",
            "STA $02",
            "LDX #$0a",
            "STX $0704",
            "LDA ($01),Y",
        ];
        let statements: Vec<Statement> =
            codes.iter().map(|c| parse_statement(c).unwrap()).collect();
        let expected: Vec<Vec<u8>> = vec![
            vec![0xa0, 0x01],
            vec![0xa9, 0x03],
            vec![0x85, 0x01],
            vec![0xa9, 0x07],
            vec![0x85, 0x02],
            vec![0xa2, 0x0a],
            vec![0x8e, 0x04, 0x07],
            vec![0xb1, 0x01],
        ];
        for (c, s, e) in izip!(codes, statements, expected) {
            assert_eq!(
                s.assemble(),
                e,
                "{} was assembled wrong, statement is {:?}",
                c,
                s
            );
        }
    }

    #[test]
    fn test_assemble_with_relative_label() {
        let code = r"
        x:
            BRK
            BRK
            BNE y
        y:
            BRK
            BNE x
        ";
        let expected_bytes_str = "00 00 d0 00 00 d0 f9";
        assert_code_assemble_to(code, expected_bytes_str);
    }

    #[test]
    fn test_assemble_with_define() {
        let code = r"
        define  sysRandom  $fe ; an address
        define  a_dozen    $0c ; a constant
       
        LDA sysRandom  ; equivalent to 'LDA $fe'
      
        LDX #a_dozen   ; equivalent to 'LDX #$0c'
        ";
        let expected_bytes_str = "a5 fe a2 0c";
        assert_code_assemble_to(code, expected_bytes_str);
    }

    #[test]
    fn test_assemble_snake_program() {
        let code = r"
        ;  ___           _        __ ___  __ ___
        ; / __|_ _  __ _| |_____ / /| __|/  \_  )
        ; \__ \ ' \/ _` | / / -_) _ \__ \ () / /
        ; |___/_||_\__,_|_\_\___\___/___/\__/___|
        
        ; Change direction: W A S D
        
        define appleL         $00 ; screen location of apple, low byte
        define appleH         $01 ; screen location of apple, high byte
        define snakeHeadL     $10 ; screen location of snake head, low byte
        define snakeHeadH     $11 ; screen location of snake head, high byte
        define snakeBodyStart $12 ; start of snake body byte pairs
        define snakeDirection $02 ; direction (possible values are below)
        define snakeLength    $03 ; snake length, in bytes
        
        ; Directions (each using a separate bit)
        define movingUp      1
        define movingRight   2
        define movingDown    4
        define movingLeft    8
        
        ; ASCII values of keys controlling the snake
        define ASCII_w      $77
        define ASCII_a      $61
        define ASCII_s      $73
        define ASCII_d      $64
        
        ; System variables
        define sysRandom    $fe
        define sysLastKey   $ff
        
        
          jsr init
          jsr loop
        
        init:
          jsr initSnake
          jsr generateApplePosition
          rts
        
        
        initSnake:
          lda #movingRight  ;start direction
          sta snakeDirection
        
          lda #4  ;start length (2 segments)
          sta snakeLength
          
          lda #$11
          sta snakeHeadL
          
          lda #$10
          sta snakeBodyStart
          
          lda #$0f
          sta $14 ; body segment 1
          
          lda #$04
          sta snakeHeadH
          sta $13 ; body segment 1
          sta $15 ; body segment 2
          rts
        
        
        generateApplePosition:
          ;load a new random byte into $00
          lda sysRandom
          sta appleL
        
          ;load a new random number from 2 to 5 into $01
          lda sysRandom
          and #$03 ;mask out lowest 2 bits
          clc
          adc #2
          sta appleH
        
          rts
        
        
        loop:
          jsr readKeys
          jsr checkCollision
          jsr updateSnake
          jsr drawApple
          jsr drawSnake
          jsr spinWheels
          jmp loop
        
        
        readKeys:
          lda sysLastKey
          cmp #ASCII_w
          beq upKey
          cmp #ASCII_d
          beq rightKey
          cmp #ASCII_s
          beq downKey
          cmp #ASCII_a
          beq leftKey
          rts
        upKey:
          lda #movingDown
          bit snakeDirection
          bne illegalMove
        
          lda #movingUp
          sta snakeDirection
          rts
        rightKey:
          lda #movingLeft
          bit snakeDirection
          bne illegalMove
        
          lda #movingRight
          sta snakeDirection
          rts
        downKey:
          lda #movingUp
          bit snakeDirection
          bne illegalMove
        
          lda #movingDown
          sta snakeDirection
          rts
        leftKey:
          lda #movingRight
          bit snakeDirection
          bne illegalMove
        
          lda #movingLeft
          sta snakeDirection
          rts
        illegalMove:
          rts
        
        
        checkCollision:
          jsr checkAppleCollision
          jsr checkSnakeCollision
          rts
        
        
        checkAppleCollision:
          lda appleL
          cmp snakeHeadL
          bne doneCheckingAppleCollision
          lda appleH
          cmp snakeHeadH
          bne doneCheckingAppleCollision
        
          ;eat apple
          inc snakeLength
          inc snakeLength ;increase length
          jsr generateApplePosition
        doneCheckingAppleCollision:
          rts
        
        
        checkSnakeCollision:
          ldx #2 ;start with second segment
        snakeCollisionLoop:
          lda snakeHeadL,x
          cmp snakeHeadL
          bne continueCollisionLoop
        
        maybeCollided:
          lda snakeHeadH,x
          cmp snakeHeadH
          beq didCollide
        
        continueCollisionLoop:
          inx
          inx
          cpx snakeLength          ;got to last section with no collision
          beq didntCollide
          jmp snakeCollisionLoop
        
        didCollide:
          jmp gameOver
        didntCollide:
          rts
        
        
        updateSnake:
          ldx snakeLength
          dex
          txa
        updateloop:
          lda snakeHeadL,x
          sta snakeBodyStart,x
          dex
          bpl updateloop
        
          lda snakeDirection
          lsr
          bcs up
          lsr
          bcs right
          lsr
          bcs down
          lsr
          bcs left
        up:
          lda snakeHeadL
          sec
          sbc #$20
          sta snakeHeadL
          bcc upup
          rts
        upup:
          dec snakeHeadH
          lda #$1
          cmp snakeHeadH
          beq collision
          rts
        right:
          inc snakeHeadL
          lda #$1f
          bit snakeHeadL
          beq collision
          rts
        down:
          lda snakeHeadL
          clc
          adc #$20
          sta snakeHeadL
          bcs downdown
          rts
        downdown:
          inc snakeHeadH
          lda #$6
          cmp snakeHeadH
          beq collision
          rts
        left:
          dec snakeHeadL
          lda snakeHeadL
          and #$1f
          cmp #$1f
          beq collision
          rts
        collision:
          jmp gameOver
        
        
        drawApple:
          ldy #0
          lda sysRandom
          sta (appleL),y
          rts
        
        
        drawSnake:
          ldx snakeLength
          lda #0
          sta (snakeHeadL,x) ; erase end of tail
        
          ldx #0
          lda #1
          sta (snakeHeadL,x) ; paint head
          rts
        
        
        spinWheels:
          ldx #0
        spinloop:
          nop
          nop
          dex
          bne spinloop
          rts
        
        
        gameOver:
        ";
        let expected_bytes_str = r"
        20 06 06 20 38 06 20 0d 06 20 2a 06 60 a9 02 85 
        02 a9 04 85 03 a9 11 85 10 a9 10 85 12 a9 0f 85 
        14 a9 04 85 11 85 13 85 15 60 a5 fe 85 00 a5 fe 
        29 03 18 69 02 85 01 60 20 4d 06 20 8d 06 20 c3 
        06 20 19 07 20 20 07 20 2d 07 4c 38 06 a5 ff c9 
        77 f0 0d c9 64 f0 14 c9 73 f0 1b c9 61 f0 22 60 
        a9 04 24 02 d0 26 a9 01 85 02 60 a9 08 24 02 d0 
        1b a9 02 85 02 60 a9 01 24 02 d0 10 a9 04 85 02 
        60 a9 02 24 02 d0 05 a9 08 85 02 60 60 20 94 06 
        20 a8 06 60 a5 00 c5 10 d0 0d a5 01 c5 11 d0 07 
        e6 03 e6 03 20 2a 06 60 a2 02 b5 10 c5 10 d0 06 
        b5 11 c5 11 f0 09 e8 e8 e4 03 f0 06 4c aa 06 4c 
        35 07 60 a6 03 ca 8a b5 10 95 12 ca 10 f9 a5 02 
        4a b0 09 4a b0 19 4a b0 1f 4a b0 2f a5 10 38 e9 
        20 85 10 90 01 60 c6 11 a9 01 c5 11 f0 28 60 e6 
        10 a9 1f 24 10 f0 1f 60 a5 10 18 69 20 85 10 b0 
        01 60 e6 11 a9 06 c5 11 f0 0c 60 c6 10 a5 10 29 
        1f c9 1f f0 01 60 4c 35 07 a0 00 a5 fe 91 00 60 
        a6 03 a9 00 81 10 a2 00 a9 01 81 10 60 a2 00 ea 
        ea ca d0 fb 60 
        ";
        assert_code_assemble_to(code, expected_bytes_str);
    }

    // ----- Helper Test Functions -----
    fn assert_code_assemble_to(code_str: &str, expected_bytes_str: &str) {
        let lines = code_str
            .split("\n")
            .into_iter()
            .map(|x| x.to_string())
            .collect();
        let assembler = Assembler::new(lines);
        let expected_bytes: Vec<u8> = expected_bytes_str
            .replace("\n", " ")
            .split(" ")
            .filter(|s| !s.is_empty())
            .map(|byte_str| u8::from_str_radix(byte_str.trim(), 16).unwrap())
            .collect();
        let assembled_bytes = assembler.assemble(0x0600u16);
        println!("Expected: {:02X?}", expected_bytes);
        println!("Actual:   {:02X?}", assembled_bytes);
        assert_eq!(assembled_bytes, expected_bytes);
    }
}

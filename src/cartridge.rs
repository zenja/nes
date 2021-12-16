use crate::mapper::mapper;

const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Debug)]
pub struct Cartridge {
    pub mapper_id: u8,
    pub mapper: Box<dyn mapper::Mapper>,
    pub mirror: Mirror,
    pub num_prg_banks: u8,
    pub num_chr_banks: u8,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
}

impl Cartridge {
    pub fn new(raw: &Vec<u8>) -> Result<Cartridge, String> {
        if &raw[0..4] != [0x4Eu8, 0x45u8, 0x53u8, 0x1Au8] {
            return Err("NES identifier not found".to_string());
        }
        let num_prg_banks = raw[4];
        let num_chr_banks = raw[5];

        let ctrl_byte_1 = raw[6];
        let ctrl_byte_2 = raw[7];

        let mapper_id = (ctrl_byte_2 & 0b1111_0000) | (ctrl_byte_1 >> 4);
        let mapper = match mapper::new(mapper_id, num_prg_banks, num_chr_banks) {
            Some(mapper) => mapper,
            None => return Err(format!("Mapper {} not supported", mapper_id).to_string()),
        };
        let mirror: Mirror = {
            if ctrl_byte_1 & (1 << 3) != 0 {
                Mirror::FourScreen
            } else if ctrl_byte_1 & (1 << 0) != 0 {
                Mirror::Vertical
            } else {
                Mirror::Horizontal
            }
        };

        // assert iNes 1.0 format
        if ctrl_byte_2 & (0b0000_1111) != 0 {
            return Err(
                "Bit 0 to 3 of control byte 2 should be zero for iNes 1.0 format".to_string(),
            );
        }

        let prg_rom_size = num_prg_banks as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = num_chr_banks as usize * CHR_ROM_PAGE_SIZE;
        let has_trainer: bool = (ctrl_byte_1 & (1 << 2)) != 0;
        let prg_rom_start = 16 + (if has_trainer { 512 } else { 0 });
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom = raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec();
        let chr_rom = raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec();

        Ok(Cartridge {
            mapper_id: mapper_id,
            mapper: mapper,
            mirror: mirror,
            num_prg_banks: num_prg_banks,
            num_chr_banks: num_chr_banks,
            prg_rom: prg_rom,
            chr_rom: chr_rom,
        })
    }

    pub fn new_from_file<P: AsRef<std::path::Path>>(ines_file: P) -> Result<Cartridge, String> {
        use std::fs;
        let raw = fs::read(&ines_file).map_err(|e| {
            format!(
                "failed to read file {}: {:?}",
                &ines_file.as_ref().display(),
                e
            )
        })?;
        Cartridge::new(&raw)
    }

    pub fn new_from_program(mut program: Vec<u8>) -> Cartridge {
        use crate::mapper::mapper_0::Mapper0;
        let min_len = 16 * 1024;
        if program.len() < min_len {
            program.resize(min_len, 0u8);
        }
        Cartridge {
            mapper_id: 0u8,
            mapper: Box::new(Mapper0::new(1, 1)),
            mirror: Mirror::Horizontal,
            num_prg_banks: 1,
            num_chr_banks: 1,
            prg_rom: program,
            chr_rom: vec![],
        }
    }

    pub fn new_dummy() -> Cartridge {
        use crate::mapper::mapper_0::Mapper0;
        Cartridge {
            mapper_id: 0u8,
            mapper: Box::new(Mapper0::new(1, 1)),
            mirror: Mirror::Horizontal,
            num_prg_banks: 1,
            num_chr_banks: 1,
            prg_rom: vec![],
            chr_rom: vec![],
        }
    }

    pub fn cpu_read(&self, addr: u16) -> Option<u8> {
        self.mapper
            .cpu_read_mapping(addr)
            .map(|a| self.prg_rom[a as usize])
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) -> bool {
        match self.mapper.cpu_read_mapping(addr) {
            Some(mapped_addr) => {
                self.prg_rom[mapped_addr as usize] = value;
                true
            }
            None => false,
        }
    }

    pub fn ppu_read(&self, addr: u16) -> Option<u8> {
        self.mapper
            .ppu_read_mapping(addr)
            .map(|a| self.chr_rom[a as usize])
    }

    pub fn ppu_write(&mut self, addr: u16, value: u8) -> bool {
        match self.mapper.ppu_read_mapping(addr) {
            Some(mapped_addr) => {
                self.chr_rom[mapped_addr as usize] = value;
                true
            }
            None => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Mirror {
    Vertical,
    Horizontal,
    FourScreen,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_nes_file() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/resources/nestest.nes");
        // let c = Cartridge::new_from_file("/Users/xing/Downloads/nestest.nes").unwrap();
        let c = Cartridge::new_from_file(p).unwrap();
        assert_eq!(c.mapper_id, 0);
        assert_eq!(c.num_prg_banks, 1);
        assert_eq!(c.num_chr_banks, 1);
        assert_eq!(c.mirror, Mirror::Horizontal);
    }
}

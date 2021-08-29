pub struct Mapper0 {
    num_prg_banks: u8,
    num_chr_banks: u8,
}

impl Mapper0 {
    pub fn new(num_prg_banks: u8, num_chr_banks: u8) -> Mapper0 {
        Mapper0 {
            num_prg_banks,
            num_chr_banks,
        }
    }
}
impl super::mapper::Mapper for Mapper0 {
    fn cpu_read_mapping(&self, addr: u16) -> (u16, bool) {
        // if PRGROM is 16KB
        //     CPU Address Bus          PRG ROM
        //     0x8000 -> 0xBFFF: Map    0x0000 -> 0x3FFF
        //     0xC000 -> 0xFFFF: Mirror 0x0000 -> 0x3FFF
        // if PRGROM is 32KB
        //     CPU Address Bus          PRG ROM
        //     0x8000 -> 0xFFFF: Map    0x0000 -> 0x7FFF
        if addr >= 0x8000 && addr <= 0xFFFF {
            let mapped_addr = addr
                & (if self.num_prg_banks > 1 {
                    0x7FFF
                } else {
                    0x3FFF
                });
            return (mapped_addr, true);
        }
        return (0u16, false);
    }

    fn cpu_write_mapping(&self, addr: u16) -> (u16, bool) {
        if addr >= 0x8000 && addr <= 0xFFFF {
            let mapped_addr = addr
                & (if self.num_prg_banks > 1 {
                    0x7FFF
                } else {
                    0x3FFF
                });
            return (mapped_addr, true);
        }
        return (0u16, false);
    }

    fn ppu_read_mapping(&self, addr: u16) -> (u16, bool) {
        // There is no mapping required for PPU
        // PPU Address Bus          CHR ROM
        // 0x0000 -> 0x1FFF: Map    0x0000 -> 0x1FFF
        if addr >= 0x0000 && addr <= 0x1FFF {
            return (addr, true);
        }
        return (0u16, false);
    }

    fn ppu_write_mapping(&self, addr: u16) -> (u16, bool) {
        if addr >= 0x0000 && addr <= 0x1FFF {
            if self.num_chr_banks == 0 {
                return (addr, true);
            }
        }
        return (0u16, false);
    }
}

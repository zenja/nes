pub mod registers;

use crate::cartridge::Mirror;
use crate::{cartridge::Cartridge, graphics::Tile};
use registers::addr::AddrRegister;
use registers::ctrl::CtrlRegister;

pub struct PPU {
    chr_rom: Vec<u8>,
    vram: [u8; 2048],
    palette_table: [u8; 32],
    mirror: Mirror,

    // registers
    addr_reg: AddrRegister,
    ctrl_reg: CtrlRegister,

    // internal data buffer
    data_buf: u8,
}

impl PPU {
    pub fn new(cart: &Cartridge) -> Self {
        PPU {
            chr_rom: cart.chr_rom.clone(),
            vram: [0; 2048],
            palette_table: [0; 32],
            mirror: cart.mirror,
            addr_reg: AddrRegister::new(),
            ctrl_reg: CtrlRegister::new(),
            data_buf: 0,
        }
    }

    pub fn load_tile(&self, bank: u32, tile_idx: u32) -> Result<Tile, String> {
        if bank != 0 && bank != 1 {
            return Err(format!("Wrong bank index: {}", bank));
        }

        // Each CHR Rom bank is 4KB
        let start = 4096 * bank as usize;
        let end = 4096 * (bank + 1) as usize;
        let bank_bytes: &[u8] = &self.chr_rom[start..end];

        let left_bytes = &bank_bytes[(tile_idx * 16) as usize..(tile_idx * 16 + 8) as usize];
        let right_bytes = &bank_bytes[(tile_idx * 16 + 8) as usize..(tile_idx * 16 + 16) as usize];
        Ok(Tile::new(left_bytes, right_bytes).unwrap())
    }

    pub fn write_addr_reg(&mut self, value: u8) {
        self.addr_reg.write(value);
    }

    pub fn write_ctrl_reg(&mut self, value: u8) {
        self.ctrl_reg.write(value);
    }

    pub fn read_data_reg(&mut self) -> u8 {
        let addr = self.addr_reg.get() & 0x3FFF;
        let buf = self.data_buf;

        // reading data reg increases addr
        self.addr_reg.inc(self.ctrl_reg.get_vram_addr_inc());

        match addr {
            // CHR Rom
            0..=0x1FFF => {
                self.data_buf = self.chr_rom[addr as usize];
                buf
            }
            // VRAM
            0x2000..=0x3EFF => {
                let mirrored = addr & 0b0000_1111_1111_1111;
                self.data_buf = self.vram[self.get_mirrored_vram_addr(mirrored) as usize];
                buf
            }
            // reading from palette table is instant - internal buffer is not involved
            0x3F00..=0x3FFF => {
                let mut mirrored = addr & 0b0000_0000_0001_1111;
                if mirrored == 0x0010 {
                    mirrored = 0x0000;
                }
                if mirrored == 0x0014 {
                    mirrored = 0x0004;
                }
                if mirrored == 0x0018 {
                    mirrored = 0x0008;
                }
                if mirrored == 0x001C {
                    mirrored = 0x000C;
                }
                // TODO consider gray scale specified in mask register
                self.palette_table[mirrored as usize]
            }
            _ => panic!(
                "reading PPU memory at address {:#06x} is not supported",
                addr
            ),
        }
    }

    pub fn write_data_reg(&mut self, value: u8) {
        let addr = self.addr_reg.get();

        // writing data reg increases addr
        self.addr_reg.inc(self.ctrl_reg.get_vram_addr_inc());

        match addr {
            // CHR Rom
            0..=0x1FFF => {
                panic!("writing to CHR Rom is not supported")
            }
            // VRAM
            0x2000..=0x3EFF => {
                let mirrored = addr & 0b0000_1111_1111_1111;
                self.vram[self.get_mirrored_vram_addr(mirrored) as usize] = value;
            }
            // reading from palette table is instant - internal buffer is not involved
            0x3F00..=0x3FFF => {
                let mut mirrored = addr & 0b0000_0000_0001_1111;
                if mirrored == 0x0010 {
                    mirrored = 0x0000;
                }
                if mirrored == 0x0014 {
                    mirrored = 0x0004;
                }
                if mirrored == 0x0018 {
                    mirrored = 0x0008;
                }
                if mirrored == 0x001C {
                    mirrored = 0x000C;
                }
                self.palette_table[mirrored as usize] = value;
            }
            _ => panic!(
                "writing PPU memory at address {:#06x} is not supported",
                addr
            ),
        }
    }

    // Horizontal:
    //   [ A ] [ A ]
    //   [ B ] [ B ]
    // Vertical:
    //   [ A ] [ B ]
    //   [ A ] [ B ]
    //
    // Return index in vram array
    fn get_mirrored_vram_addr(&self, addr: u16) -> u16 {
        // From [0x2000 - 0x3EFF) to [0x0000, 0x0FFF] (4K),
        // which represents vram for 4 nametables
        let logical_vram_idx = addr & 0b0000_1111_1111_1111;
        // each nametable is 1K (0x0400 bytes)
        // nametable_idx is 0/1/2/3
        let nametable_idx = logical_vram_idx / 0x0400;
        let vram_idx_a: u16 = logical_vram_idx % 0x0400;
        let vram_idx_b: u16 = vram_idx_a + 0x0400;
        match (self.mirror, nametable_idx) {
            // A - the 1st physical nametable
            (Mirror::Horizontal, 0)
            | (Mirror::Horizontal, 1)
            | (Mirror::Vertical, 0)
            | (Mirror::Vertical, 2) => vram_idx_a,
            // B - the 2nd physical nametable
            (Mirror::Horizontal, 2)
            | (Mirror::Horizontal, 3)
            | (Mirror::Vertical, 1)
            | (Mirror::Vertical, 3) => vram_idx_b,
            // TODO more kinds of mirroring?
            _ => logical_vram_idx,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_ppu() -> PPU {
        let cart = Cartridge::new_dummy();
        PPU::new(&cart)
    }

    #[test]
    fn test_write_vram() {
        let mut ppu = new_ppu();
        ppu.write_addr_reg(0x23);
        ppu.write_addr_reg(0x05);
        ppu.write_data_reg(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_read_vram() {
        let mut ppu = new_ppu();
        ppu.write_ctrl_reg(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_addr_reg(0x23);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load_into_buffer
        assert_eq!(ppu.addr_reg.get(), 0x2306);
        assert_eq!(ppu.read_data_reg(), 0x66);
    }

    #[test]
    fn test_read_vram_cross_page() {
        let mut ppu = new_ppu();
        ppu.write_ctrl_reg(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_addr_reg(0x21);
        ppu.write_addr_reg(0xff);

        ppu.read_data_reg(); // load_into_buffer
        assert_eq!(ppu.read_data_reg(), 0x66);
        assert_eq!(ppu.read_data_reg(), 0x77);
    }

    #[test]
    fn test_read_vram_step_32() {
        let mut ppu = new_ppu();
        ppu.write_ctrl_reg(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_addr_reg(0x21);
        ppu.write_addr_reg(0xff);

        ppu.read_data_reg(); // load_into_buffer
        assert_eq!(ppu.read_data_reg(), 0x66);
        assert_eq!(ppu.read_data_reg(), 0x77);
        assert_eq!(ppu.read_data_reg(), 0x88);
    }

    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = new_ppu();
        ppu.mirror = Mirror::Horizontal;

        ppu.write_addr_reg(0x24);
        ppu.write_addr_reg(0x05);

        ppu.write_data_reg(0x66); // write to a

        ppu.write_addr_reg(0x28);
        ppu.write_addr_reg(0x05);

        ppu.write_data_reg(0x77); // write to B

        ppu.write_addr_reg(0x20);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load into buffer
        assert_eq!(ppu.read_data_reg(), 0x66); // read from A

        ppu.write_addr_reg(0x2C);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load into buffer
        assert_eq!(ppu.read_data_reg(), 0x77); // read from b
    }

    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = new_ppu();
        ppu.mirror = Mirror::Vertical;

        ppu.write_addr_reg(0x20);
        ppu.write_addr_reg(0x05);

        ppu.write_data_reg(0x66); // write to A

        ppu.write_addr_reg(0x2C);
        ppu.write_addr_reg(0x05);

        ppu.write_data_reg(0x77); // write to b

        ppu.write_addr_reg(0x28);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load into buffer
        assert_eq!(ppu.read_data_reg(), 0x66); // read from a

        ppu.write_addr_reg(0x24);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load into buffer
        assert_eq!(ppu.read_data_reg(), 0x77); // read from B
    }

    #[test]
    fn test_vram_mirroring() {
        let mut ppu = new_ppu();

        ppu.write_ctrl_reg(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_addr_reg(0x63); // 0x6305 -> 0x2305
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load into_buffer
        assert_eq!(ppu.read_data_reg(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }
}

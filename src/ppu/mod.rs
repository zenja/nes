pub mod registers;

use crate::cartridge::Mirror;
use crate::graphics::{self, NesFrame, Palette};
use crate::{cartridge::Cartridge, graphics::Tile};
use registers::addr::AddrRegister;
use registers::ctrl::CtrlRegister;

use self::registers::mask::MaskRegister;
use self::registers::scroll::ScrollRegister;
use self::registers::status::StatusRegister;

pub struct PPU {
    chr_rom: Vec<u8>,
    vram: [u8; 2048],
    palette_table: [u8; 32],
    mirror: Mirror,

    // registers
    addr_reg: AddrRegister,
    ctrl_reg: CtrlRegister,
    status_reg: StatusRegister,
    scroll_reg: ScrollRegister,
    mask_reg: MaskRegister,

    // internal data buffer
    data_buf: u8,

    // NMI status
    nmi: bool,

    // temp field for tracking PPU cycles and scanlines
    scanlines: u32,
    cycles: u32,
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
            status_reg: StatusRegister::new(),
            scroll_reg: ScrollRegister::new(),
            mask_reg: MaskRegister::new(),
            data_buf: 0,
            nmi: false,
            scanlines: 0,
            cycles: 0,
        }
    }

    pub fn tick(&mut self) {
        // TODO handle status register change

        self.cycles += 1;
        if self.cycles == 341 {
            self.cycles = 0;
            self.scanlines += 1;

            if self.scanlines == 241 {
                self.status_reg.set_vblank_started(true);
                if self.ctrl_reg.is_generate_nmi() {
                    self.nmi = true;
                }
            }

            if self.scanlines == 262 {
                self.scanlines = 0;
                self.status_reg.set_vblank_started(false);
                self.nmi = false;
            }
        }
    }

    pub fn cpu_read(&mut self, cpu_addr: u16) -> u8 {
        match cpu_addr {
            0x2000..=0x3FFF => match cpu_addr & 0x0007 {
                // Ctrl register (write-only)
                0x0000 => 0,
                // Mask register (write-only)
                0x0001 => 0,
                // Status register
                0x0002 => self.read_status_reg(),
                // OAM address register (write-only)
                0x0003 => 0,
                // OAM data register
                0x0004 => 0, // TODO
                // Scroll register (write-only)
                0x0005 => 0,
                // PPU address register (write-only)
                0x0006 => 0,
                // PPU data register
                0x0007 => self.read_data_reg(),
                _ => panic!("impossible"),
            },
            _ => panic!("CPU read address {:04X?} not supported for PPU!", cpu_addr),
        }
    }

    pub fn cpu_write(&mut self, cpu_addr: u16, value: u8) {
        match cpu_addr {
            0x2000..=0x3FFF => match cpu_addr & 0x0007 {
                // Ctrl register
                0x0000 => self.write_ctrl_reg(value),
                // Mask register
                0x0001 => self.write_mask_reg(value),
                // Status register
                0x0002 => panic!("PPU status register is not writable!"),
                // OAM address register
                0x0003 => (), // TODO
                // OAM data register
                0x0004 => (), // TODO
                // Scroll register
                0x0005 => self.write_scroll_reg(value),
                // PPU address register
                0x0006 => self.write_addr_reg(value),
                // PPU data register
                0x0007 => self.write_data_reg(value),
                _ => panic!("impossible"),
            },
            _ => panic!("CPU write address {:04X?} not supported for PPU!", cpu_addr),
        }
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
                // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
                // Addresses $3F04/$3F08/$3F0C can contain unique data,
                // though these values are not used by the PPU when normally rendering
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
                if self.mask_reg.grayscale() {
                    self.palette_table[mirrored as usize] & 0x30
                } else {
                    self.palette_table[mirrored as usize] & 0x3F
                }
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
            // palette table
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

    pub fn read_status_reg(&mut self) -> u8 {
        let value = self.status_reg.read();
        // reading status register changes some status
        self.status_reg.set_vblank_started(false);
        self.addr_reg.reset_latch();
        self.scroll_reg.reset_latch();
        value
    }

    pub fn write_mask_reg(&mut self, value: u8) {
        self.mask_reg.write(value);
    }

    pub fn write_scroll_reg(&mut self, value: u8) {
        self.scroll_reg.write(value);
    }

    pub fn has_nmi(&self) -> bool {
        self.nmi
    }

    pub fn reset_nmi(&mut self) {
        self.nmi = false;
    }

    pub fn render_ppu(&self, frame: &mut NesFrame) {
        let nametable_addr = self.ctrl_reg.get_base_nametable_addr();
        for tile_y in 0..30 {
            for tile_x in 0..32 {
                let tile_idx = self.vram
                    [self.get_mirrored_vram_addr(nametable_addr + tile_y * 32 + tile_x) as usize];
                let tile = self
                    .load_tile(
                        self.ctrl_reg.get_background_pattern_table_bank() as u8,
                        tile_idx,
                    )
                    .unwrap();
                let palette = self.load_bg_palette(tile_x as u8, tile_y as u8);
                frame.draw_tile(false, tile_x as u32 * 8, tile_y as u32 * 8, &tile, &palette);
            }
        }
    }

    pub fn load_tile(&self, bank: u8, tile_idx: u8) -> Result<Tile, String> {
        if bank != 0 && bank != 1 {
            return Err(format!("Wrong bank index: {}", bank));
        }

        // Each CHR Rom bank is 4KB
        let start = 4096 * bank as usize;
        let end = 4096 * (bank + 1) as usize;
        let bank_bytes: &[u8] = &self.chr_rom[start..end];

        let left_bytes = &bank_bytes[(tile_idx as usize * 16)..(tile_idx as usize * 16 + 8)];
        let right_bytes = &bank_bytes[(tile_idx as usize * 16 + 8)..(tile_idx as usize * 16 + 16)];
        Ok(Tile::new(left_bytes, right_bytes).unwrap())
    }

    fn load_bg_palette(&self, tile_x: u8, tile_y: u8) -> Palette {
        let nametable_addr = self.ctrl_reg.get_base_nametable_addr();
        let attr_table_addr = nametable_addr + 960;
        let block_x = tile_x / 4;
        let block_y = tile_y / 4;
        // the attribute table record for this block
        let block_attr = self.vram[self
            .get_mirrored_vram_addr(attr_table_addr + block_y as u16 * 8 + block_x as u16)
            as usize];
        // index of which palette (out of 4 possible palettes)
        let logical_palette_idx: u8 = match ((tile_x % 4) / 2, (tile_y % 4) / 2) {
            (0, 0) => (block_attr & 0b00_00_00_11) >> 0,
            (1, 0) => (block_attr & 0b00_00_11_00) >> 2,
            (0, 1) => (block_attr & 0b00_11_00_00) >> 4,
            (1, 1) => (block_attr & 0b11_00_00_00) >> 6,
            (_, _) => panic!("impossible!"),
        };
        let palette_arr_start = 1 + logical_palette_idx as usize * 4;
        Palette {
            colors: [
                graphics::SYSTEM_PALETTE[self.palette_table[0] as usize],
                graphics::SYSTEM_PALETTE[self.palette_table[palette_arr_start] as usize],
                graphics::SYSTEM_PALETTE[self.palette_table[palette_arr_start + 1] as usize],
                graphics::SYSTEM_PALETTE[self.palette_table[palette_arr_start + 2] as usize],
            ],
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

        assert_eq!(ppu.vram[ppu.get_mirrored_vram_addr(0x2305) as usize], 0x66);
    }

    #[test]
    fn test_read_vram() {
        let mut ppu = new_ppu();
        ppu.write_ctrl_reg(0);
        ppu.vram[ppu.get_mirrored_vram_addr(0x2305) as usize] = 0x66;

        ppu.write_addr_reg(0x23);
        ppu.write_addr_reg(0x05);

        ppu.read_data_reg(); // load_into_buffer
        assert_eq!(ppu.addr_reg.get(), 0x2306);
        assert_eq!(ppu.read_data_reg(), 0x66);
    }

    #[test]
    fn test_write_then_read_vram() {
        let mut ppu = new_ppu();
        ppu.write_addr_reg(0x20);
        ppu.write_addr_reg(0x00);
        // 0x2000 => 0x00
        ppu.write_data_reg(0x00);
        // 0x2001 => 0x01
        ppu.write_data_reg(0x01);
        // 0x2002 => 0x02
        ppu.write_data_reg(0x02);

        ppu.write_addr_reg(0x20);
        ppu.write_addr_reg(0x00);
        ppu.read_data_reg();
        assert_eq!(ppu.read_data_reg(), 0x00);
        assert_eq!(ppu.read_data_reg(), 0x01);
        assert_eq!(ppu.read_data_reg(), 0x02);
        assert_eq!(ppu.read_data_reg(), 0x00);
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

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = new_ppu();
        ppu.status_reg.set_vblank_started(true);

        let status = ppu.read_status_reg();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status_reg.read() >> 7, 0);
    }
}

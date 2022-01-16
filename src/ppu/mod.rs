pub mod registers;

use crate::cartridge::Cartridge;
use crate::cartridge::Mirror;
use crate::graphics::NesFrame;
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

    // OAM
    pub oam_data: [u8; 256],
    oam_addr: u8,

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
            oam_data: [0; 256],
            oam_addr: 0,
            data_buf: 0,
            nmi: false,
            scanlines: 0,
            cycles: 0,
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        if self.cycles == 341 {
            if self.is_sprite_zero_hit() {
                self.status_reg.set_sprite_zero_hit(true);
            }

            self.cycles = 0;
            self.scanlines += 1;

            if self.scanlines == 241 {
                self.status_reg.set_vblank_started(true);
                // the sprite zero hit flag should be erased upon entering VBLANK state
                self.status_reg.set_sprite_zero_hit(false);
                if self.ctrl_reg.is_generate_nmi() {
                    self.nmi = true;
                }
            }

            if self.scanlines == 262 {
                self.scanlines = 0;
                self.status_reg.set_vblank_started(false);
                self.status_reg.set_sprite_zero_hit(false);
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
                0x0004 => self.read_oam_data(),
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
                0x0003 => self.write_oam_addr(value),
                // OAM data register
                0x0004 => self.write_oam_data(value),
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

    pub fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn write_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr += 1;
    }

    pub fn has_nmi(&self) -> bool {
        self.nmi
    }

    pub fn reset_nmi(&mut self) {
        self.nmi = false;
    }

    pub fn render_ppu(&self, frame: &mut NesFrame) {
        self.render_background(frame);
        self.render_sprites(frame);
    }

    pub fn render_background(&self, frame: &mut NesFrame) {
        let scroll_x = (self.scroll_reg.scroll_x) as usize;
        let scroll_y = (self.scroll_reg.scroll_y) as usize;

        let (main_nametable_addr, second_nametable_addr) =
            match (&self.mirror, self.ctrl_reg.get_base_nametable_addr()) {
                (Mirror::Vertical, 0x2000)
                | (Mirror::Vertical, 0x2800)
                | (Mirror::Horizontal, 0x2000)
                | (Mirror::Horizontal, 0x2400) => (0x0000u16, 0x0400u16),
                (Mirror::Vertical, 0x2400)
                | (Mirror::Vertical, 0x2C00)
                | (Mirror::Horizontal, 0x2800)
                | (Mirror::Horizontal, 0x2C00) => (0x0400u16, 0x0000u16),
                (_, _) => {
                    panic!("Not supported mirroring type {:?}", self.mirror);
                }
            };

        self.render_nametable(
            frame,
            main_nametable_addr,
            &Rect::new(scroll_x, scroll_y, 256, 240),
            -(scroll_x as i32),
            -(scroll_y as i32),
        );
        if scroll_x > 0 {
            self.render_nametable(
                frame,
                second_nametable_addr,
                &Rect::new(0, 0, scroll_x, 240),
                (256 - scroll_x) as i32,
                0,
            );
        } else if scroll_y > 0 {
            self.render_nametable(
                frame,
                second_nametable_addr,
                &Rect::new(0, 0, 256, scroll_y),
                0,
                (240 - scroll_y) as i32,
            );
        }
    }

    fn render_nametable(
        &self,
        frame: &mut NesFrame,
        nametable_addr: u16,
        viewport: &Rect,
        shift_x: i32,
        shift_y: i32,
    ) {
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
                let palette = self.load_bg_palette(nametable_addr, tile_x as u8, tile_y as u8);
                self.render_tile(
                    frame,
                    false,
                    tile_x as u32 * 8,
                    tile_y as u32 * 8,
                    &tile,
                    &palette,
                    viewport,
                    shift_x,
                    shift_y,
                );
            }
        }
    }

    pub fn render_tile(
        &self,
        frame: &mut NesFrame,
        is_sprite_tile: bool,
        x: u32,
        y: u32,
        tile: &Tile,
        palette: &Palette,
        viewport: &Rect,
        shift_x: i32,
        shift_y: i32,
    ) {
        // i: row index (y)
        for i in 0..8 {
            // j: column index (x)
            for j in 0..8 {
                let color_idx = tile.rows[i][j];
                let color = palette.colors[color_idx as usize];
                // do not draw background color (index 0) for sprite tiles as they should be "transparent"
                if !(is_sprite_tile && color_idx == 0) {
                    if x >= viewport.x1 as u32
                        && x <= viewport.x2 as u32
                        && y >= viewport.y1 as u32
                        && y <= viewport.y2 as u32
                    {
                        let pixel_x = x as i64 + j as i64 + shift_x as i64;
                        let pixel_x: u32 = if pixel_x < 0 { 0 } else { pixel_x as u32 };
                        let pixel_y = y as i64 + i as i64 + shift_y as i64;
                        let pixel_y: u32 = if pixel_y < 0 { 0 } else { pixel_y as u32 };
                        frame.set_pixel(pixel_x, pixel_y, color.0, color.1, color.2)
                    }
                }
            }
        }
    }

    pub fn render_sprites(&self, frame: &mut NesFrame) {
        for sid in (0..self.oam_data.len()).step_by(4) {
            // raw sprite info
            let sprite_y = self.oam_data[sid];
            let tile_idx = self.oam_data[sid + 1];
            let attr = self.oam_data[sid + 2];
            let sprite_x = self.oam_data[sid + 3];

            // detailed attributes
            let flip_vertical: bool = attr >> 7 == 1;
            let flip_horizontal: bool = attr >> 6 == 1;
            let palette_idx: u8 = attr & 0b11; // 0/1/2/3

            let palette = self.load_sprite_palette(palette_idx);
            let mut tile = self
                .load_tile(
                    self.ctrl_reg.get_sprite_pattern_table_bank() as u8,
                    tile_idx,
                )
                .unwrap();
            if flip_vertical {
                tile.flip_vertical();
            }
            if flip_horizontal {
                tile.flip_horizontal();
            }
            self.render_tile(
                frame,
                true,
                sprite_x as u32,
                sprite_y as u32,
                &tile,
                &palette,
                &Rect::new(0, 0, 256, 240),
                0,
                0,
            );
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

        let low_bytes = &bank_bytes[(tile_idx as usize * 16)..(tile_idx as usize * 16 + 8)];
        let high_bytes = &bank_bytes[(tile_idx as usize * 16 + 8)..(tile_idx as usize * 16 + 16)];
        Ok(Tile::new(low_bytes, high_bytes).unwrap())
    }

    fn load_bg_palette(&self, nametable_addr: u16, tile_x: u8, tile_y: u8) -> Palette {
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
                SYSTEM_PALETTE[self.palette_table[0] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start + 1] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start + 2] as usize],
            ],
        }
    }

    fn load_sprite_palette(&self, palette_idx: u8) -> Palette {
        let palette_arr_start: usize = 16 + 1 + palette_idx as usize * 4;
        Palette {
            colors: [
                SYSTEM_PALETTE[self.palette_table[0] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start + 1] as usize],
                SYSTEM_PALETTE[self.palette_table[palette_arr_start + 2] as usize],
            ],
        }
    }

    fn is_sprite_zero_hit(&self) -> bool {
        let y = self.oam_data[0];
        let x = self.oam_data[3];
        (y as u32 == self.scanlines)
            && (x as u32 <= self.cycles)
            && self.mask_reg.show_background()
            && self.mask_reg.show_sprites()
    }

    pub fn print_debug_info(&self) {
        println!(
            "================================================================================"
        );

        // Placeholder :)

        println!(
            "================================================================================"
        );
    }
}

// ----------------------------------------------------------------------------
// Rect
// ----------------------------------------------------------------------------

pub struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect {
            x1: x1,
            y1: y1,
            x2: x2,
            y2: y2,
        }
    }
}

// ----------------------------------------------------------------------------
// Tile
// ----------------------------------------------------------------------------

pub struct Tile {
    pub rows: [[u8; 8]; 8],
}

impl Tile {
    pub fn new(low_bytes: &[u8], high_bytes: &[u8]) -> Result<Tile, String> {
        if low_bytes.len() != 8 || high_bytes.len() != 8 {
            return Err(format!(
                "Length of low bytes and high bytes of a tile should be both 8 but are {} and {}",
                low_bytes.len(),
                high_bytes.len()
            ));
        }

        let mut rows = [[0; 8]; 8];
        for i in 0..8 {
            for j in 0..8 {
                let low_bit = (low_bytes[i] >> j) & 1;
                let high_bit = (high_bytes[i] >> j) & 1;
                rows[i][7 - j] = (high_bit << 1) + low_bit;
            }
        }
        Ok(Tile { rows: rows })
    }

    pub fn flip_vertical(&mut self) {
        for y in 0..4 {
            for x in 0..8 {
                let tmp = self.rows[y][x];
                self.rows[y][x] = self.rows[7 - y][x];
                self.rows[7 - y][x] = tmp;
            }
        }
    }

    pub fn flip_horizontal(&mut self) {
        for x in 0..4 {
            for y in 0..8 {
                let tmp = self.rows[y][x];
                self.rows[y][x] = self.rows[y][7 - x];
                self.rows[y][7 - x] = tmp;
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Palette
// ----------------------------------------------------------------------------

#[rustfmt::skip]
pub static SYSTEM_PALETTE: [(u8, u8, u8); 64] = [
    (0x80, 0x80, 0x80), (0x00, 0x3D, 0xA6), (0x00, 0x12, 0xB0), (0x44, 0x00, 0x96), (0xA1, 0x00, 0x5E),
    (0xC7, 0x00, 0x28), (0xBA, 0x06, 0x00), (0x8C, 0x17, 0x00), (0x5C, 0x2F, 0x00), (0x10, 0x45, 0x00),
    (0x05, 0x4A, 0x00), (0x00, 0x47, 0x2E), (0x00, 0x41, 0x66), (0x00, 0x00, 0x00), (0x05, 0x05, 0x05),
    (0x05, 0x05, 0x05), (0xC7, 0xC7, 0xC7), (0x00, 0x77, 0xFF), (0x21, 0x55, 0xFF), (0x82, 0x37, 0xFA),
    (0xEB, 0x2F, 0xB5), (0xFF, 0x29, 0x50), (0xFF, 0x22, 0x00), (0xD6, 0x32, 0x00), (0xC4, 0x62, 0x00),
    (0x35, 0x80, 0x00), (0x05, 0x8F, 0x00), (0x00, 0x8A, 0x55), (0x00, 0x99, 0xCC), (0x21, 0x21, 0x21),
    (0x09, 0x09, 0x09), (0x09, 0x09, 0x09), (0xFF, 0xFF, 0xFF), (0x0F, 0xD7, 0xFF), (0x69, 0xA2, 0xFF),
    (0xD4, 0x80, 0xFF), (0xFF, 0x45, 0xF3), (0xFF, 0x61, 0x8B), (0xFF, 0x88, 0x33), (0xFF, 0x9C, 0x12),
    (0xFA, 0xBC, 0x20), (0x9F, 0xE3, 0x0E), (0x2B, 0xF0, 0x35), (0x0C, 0xF0, 0xA4), (0x05, 0xFB, 0xFF),
    (0x5E, 0x5E, 0x5E), (0x0D, 0x0D, 0x0D), (0x0D, 0x0D, 0x0D), (0xFF, 0xFF, 0xFF), (0xA6, 0xFC, 0xFF),
    (0xB3, 0xEC, 0xFF), (0xDA, 0xAB, 0xEB), (0xFF, 0xA8, 0xF9), (0xFF, 0xAB, 0xB3), (0xFF, 0xD2, 0xB0),
    (0xFF, 0xEF, 0xA6), (0xFF, 0xF7, 0x9C), (0xD7, 0xE8, 0x95), (0xA6, 0xED, 0xAF), (0xA2, 0xF2, 0xDA),
    (0x99, 0xFF, 0xFC), (0xDD, 0xDD, 0xDD), (0x11, 0x11, 0x11), (0x11, 0x11, 0x11)
];

pub struct Palette {
    pub colors: [(u8, u8, u8); 4],
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

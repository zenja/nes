use crate::{cartridge::Cartridge, graphics::Tile};

pub struct PPU {}

impl PPU {
    pub fn new() -> Self {
        PPU {}
    }

    pub fn load_tile(&self, cart: &Cartridge, bank: u32, tile_idx: u32) -> Result<Tile, String> {
        if bank != 0 && bank != 1 {
            return Err(format!("Wrong bank index: {}", bank));
        }

        // Each CHR Rom bank is 4KB
        let start = 4096 * bank as usize;
        let end = 4096 * (bank + 1) as usize;
        let bank_bytes: &[u8] = &cart.chr_rom[start..end];

        let left_bytes = &bank_bytes[(tile_idx * 16) as usize..(tile_idx * 16 + 8) as usize];
        let right_bytes = &bank_bytes[(tile_idx * 16 + 8) as usize..(tile_idx * 16 + 16) as usize];
        Ok(Tile::new(left_bytes, right_bytes).unwrap())
    }
}

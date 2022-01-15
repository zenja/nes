use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::VideoSubsystem;
use std::ops::{Deref, DerefMut};

const NES_WIDTH: u32 = 32 * 8;
const NES_HEIGHT: u32 = 30 * 8;

// ----------------------------------------------------------------------------
// NesSDLScreen
// ----------------------------------------------------------------------------

pub struct NesSDLScreen {
    canvas: WindowCanvas,
    scaling_factor: u32,
}

impl NesSDLScreen {
    pub fn new(video: &VideoSubsystem, scaling_factor: u32) -> NesSDLScreen {
        let window = video
            .window(
                "NES",
                NES_WIDTH * scaling_factor,
                NES_HEIGHT * scaling_factor,
            )
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();
        let canvas = window
            .into_canvas()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();
        NesSDLScreen {
            canvas: canvas,
            scaling_factor: scaling_factor,
        }
    }

    pub fn draw(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let prev_color = self.canvas.draw_color();
        self.canvas.set_draw_color(Color::RGB(r, g, b));
        self.canvas
            .fill_rect(Rect::new(
                (x * self.scaling_factor) as i32,
                (y * self.scaling_factor) as i32,
                self.scaling_factor,
                self.scaling_factor,
            ))
            .unwrap();
        self.canvas.set_draw_color(prev_color);
    }

    pub fn draw_frame(&mut self, frame: &NesFrame) {
        for (y, row) in frame.pixels.iter().enumerate() {
            for (x, color) in row.iter().enumerate() {
                self.draw(x as u32, y as u32, color[0], color[1], color[2]);
            }
        }
    }
}

impl Deref for NesSDLScreen {
    type Target = WindowCanvas;

    fn deref(&self) -> &Self::Target {
        &self.canvas
    }
}

impl DerefMut for NesSDLScreen {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.canvas
    }
}

// ----------------------------------------------------------------------------
// NesFrame
// ----------------------------------------------------------------------------

pub struct NesFrame {
    pixels: [[[u8; 3]; NES_WIDTH as usize]; NES_HEIGHT as usize],
}

impl NesFrame {
    pub fn new() -> NesFrame {
        NesFrame {
            pixels: [[[0; 3]; NES_WIDTH as usize]; NES_HEIGHT as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= NES_WIDTH || y >= NES_HEIGHT {
            return;
        }
        self.pixels[y as usize][x as usize] = [r, g, b]
    }

    pub fn draw_tile(
        &mut self,
        is_sprite_tile: bool,
        x: u32,
        y: u32,
        tile: &Tile,
        palette: &Palette,
    ) {
        // i: row index (y)
        for i in 0..8 {
            // j: column index (x)
            for j in 0..8 {
                let color_idx = tile.rows[i][j];
                let color = palette.colors[color_idx as usize];
                // do not draw background color (index 0) for sprite tiles as they should be "transparent"
                if !(is_sprite_tile && color_idx == 0) {
                    self.set_pixel(x + j as u32, y + i as u32, color.0, color.1, color.2)
                }
            }
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

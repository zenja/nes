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
        for (x, row) in frame.pixels.iter().enumerate() {
            for (y, color) in row.iter().enumerate() {
                self.draw(x as u32, y as u32, color.0, color.1, color.2);
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
    pixels: [[(u8, u8, u8); NES_WIDTH as usize]; NES_HEIGHT as usize],
}

impl NesFrame {
    pub fn new() -> NesFrame {
        NesFrame {
            pixels: [[(0u8, 0u8, 0u8); NES_WIDTH as usize]; NES_HEIGHT as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= NES_WIDTH || y >= NES_HEIGHT {
            return;
        }
        self.pixels[x as usize][y as usize] = (r, g, b)
    }

    pub fn draw_tile(
        &mut self,
        is_sprite_tile: bool,
        x: u32,
        y: u32,
        tile: &Tile,
        palette: &Palette,
    ) {
        for i in 0..64 {
            let color_idx = tile.rows[(i % 8) as usize][(i / 8) as usize];
            let color = palette.colors[color_idx as usize];
            // do not draw background color (index 0) for sprite tiles as they should be "transparent"
            if !(is_sprite_tile && color_idx == 0) {
                self.set_pixel(x + i / 8, y + i % 8, color.0, color.1, color.2)
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
    pub fn new(left_bits: &[u8], right_bits: &[u8]) -> Result<Tile, String> {
        if left_bits.len() != 64 || right_bits.len() != 64 {
            return Err(format!(
                "Length of left bits and right bits should be both 64 but are {} and {}",
                left_bits.len(),
                right_bits.len()
            ));
        }

        let mut rows = [[0; 8]; 8];
        for i in 0..64 {
            if left_bits[i] > 1 {
                return Err(format!("Invalid bit in left: {}", left_bits[i]));
            }
            if right_bits[i] > 1 {
                return Err(format!("Invalid bit in right: {}", right_bits[i]));
            }
            rows[i / 8][i % 8] = left_bits[i] + right_bits[i];
        }
        Ok(Tile { rows: rows })
    }

    pub fn with_full_bits(bits: &[u8]) -> Result<Tile, String> {
        if bits.len() != 64 {
            return Err(format!("Length of bits should be 64 but is {}", bits.len()));
        }

        let mut rows = [[0; 8]; 8];
        for i in 0..64 {
            if bits[i] > 3 {
                return Err(format!("Invalid bit: {}", bits[i]));
            }
            rows[i / 8][i % 8] = bits[i];
        }
        Ok(Tile { rows: rows })
    }
}

// ----------------------------------------------------------------------------
// Palette
// ----------------------------------------------------------------------------

pub struct Palette {
    pub colors: [(u8, u8, u8); 4],
}

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::VideoSubsystem;
use std::ops::{Deref, DerefMut};

const NES_WIDTH: u32 = 32 * 8;
const NES_HEIGHT: u32 = 30 * 8;

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

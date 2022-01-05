extern crate nes;
extern crate sdl2;

use nes::graphics::{NesFrame, NesSDLScreen};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let mut screen = NesSDLScreen::new(&video_subsystem, 4);

    screen.set_draw_color(Color::RGB(255, 255, 255));
    screen.clear();
    screen.present();

    let mut frame = NesFrame::new();
    frame.set_pixel(50, 100, 255, 0, 0);
    frame.set_pixel(100, 150, 0, 255, 0);
    frame.set_pixel(150, 200, 0, 0, 255);

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        screen.clear();
        screen.draw_frame(&frame);
        screen.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        // The rest of the game loop goes here...
    }

    Ok(())
}

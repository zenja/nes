extern crate nes;
extern crate sdl2;

use std::path::PathBuf;
use std::time::Duration;

use nes::bus::Bus;
use nes::cartridge::Cartridge;
use nes::cpu::CPU;
use nes::graphics::{NesFrame, NesSDLScreen, Palette};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

fn main() -> Result<(), String> {
    let mut nes_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    nes_path.push("tests/resources/pacman.nes");
    let cart = Cartridge::new_from_file(nes_path).unwrap();
    let bus = Bus::new(cart);
    let mut cpu = CPU::new(bus);
    cpu.reset();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let mut screen = NesSDLScreen::new(&video_subsystem, 4);

    screen.set_draw_color(Color::RGB(255, 255, 255));
    screen.clear();
    screen.present();

    let palette = Palette {
        colors: [
            nes::graphics::SYSTEM_PALLETE[0x01],
            nes::graphics::SYSTEM_PALLETE[0x23],
            nes::graphics::SYSTEM_PALLETE[0x27],
            nes::graphics::SYSTEM_PALLETE[0x30],
        ],
    };
    let mut frame = NesFrame::new();
    let cart = &cpu.bus.cart;
    // draw for bank 0
    for i in 0..256 {
        let tile = cpu.bus.ppu.load_tile(cart, 0, i).unwrap();
        let x = (i % 32) * 8;
        let y = (i / 32) * 8;
        frame.draw_tile(false, x, y, &tile, &palette);
    }
    // draw for bank 1
    for i in 0..256 {
        let tile = cpu.bus.ppu.load_tile(cart, 1, i).unwrap();
        let x = (i % 32) * 8;
        let y = 100 + (i / 32) * 8;
        frame.draw_tile(false, x, y, &tile, &palette);
    }

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

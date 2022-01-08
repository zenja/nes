use std::path::PathBuf;
use std::time::Duration;

use cpu::CPU;
use nes::bus::Bus;
use nes::cartridge::Cartridge;
use nes::cpu;
use nes::graphics::{NesFrame, NesSDLScreen};
use nes::ppu::PPU;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let mut screen = NesSDLScreen::new(&video_subsystem, 3);
    let mut frame = NesFrame::new();
    let mut event_pump = sdl_context.event_pump()?;

    let mut nes_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    nes_path.push("tests/resources/donkey-kong.nes");
    let cart = Cartridge::new_from_file(nes_path).unwrap();
    let bus = Bus::new_with_gameloop_callback(cart, move |ppu: &PPU| {
        ppu.render_ppu(&mut frame);
        screen.clear();
        screen.draw_frame(&frame);
        screen.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => ppu.debug(),
                _ => {}
            }
        }

        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 1789773u32));
    });
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();

    Ok(())
}

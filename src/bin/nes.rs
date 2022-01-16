use std::collections::HashMap;
use std::path::PathBuf;

use cpu::CPU;
use nes::bus::Bus;
use nes::cartridge::Cartridge;
use nes::cpu;
use nes::graphics::{NesFrame, NesSDLScreen};
use nes::joypad::{Joypad, JoypadStatus};
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
    nes_path.push("tests/resources/smb.nes");
    let cart = Cartridge::new_from_file(nes_path).unwrap();
    let bus = Bus::new_with_gameloop_callback(cart, move |ppu: &PPU, joypads: &mut [Joypad; 2]| {
        ppu.render_ppu(&mut frame);
        screen.clear();
        screen.draw_frame(&frame);
        screen.present();

        let mut key_map = HashMap::new();
        key_map.insert(Keycode::Up, JoypadStatus::UP);
        key_map.insert(Keycode::Down, JoypadStatus::DOWN);
        key_map.insert(Keycode::Left, JoypadStatus::LEFT);
        key_map.insert(Keycode::Right, JoypadStatus::RIGHT);
        key_map.insert(Keycode::Space, JoypadStatus::SELECT);
        key_map.insert(Keycode::Return, JoypadStatus::START);
        key_map.insert(Keycode::A, JoypadStatus::BUTTON_A);
        key_map.insert(Keycode::S, JoypadStatus::BUTTON_B);

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
                } => ppu.print_debug_info(),
                Event::KeyDown { keycode, .. } => {
                    if let Some(btn) = key_map.get(&keycode.unwrap_or(Keycode::Escape)) {
                        joypads[0].set(btn);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(btn) = key_map.get(&keycode.unwrap_or(Keycode::Escape)) {
                        joypads[0].unset(btn);
                    }
                }
                _ => {}
            }
        }
    });
    let mut cpu = CPU::new_with_nes_clock_rate(bus);
    cpu.reset();
    cpu.run();

    Ok(())
}

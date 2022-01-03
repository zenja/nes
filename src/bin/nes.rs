use std::path::PathBuf;

use cpu::CPU;
use nes::bus::Bus;
use nes::cartridge::Cartridge;
use nes::cpu;

fn main() {
    let mut nes_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    nes_path.push("tests/resources/nestest.nes");

    let cart = Cartridge::new_from_file(nes_path).unwrap();
    let bus = Bus::new(cart);
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}

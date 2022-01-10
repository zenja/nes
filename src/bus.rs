use crate::cartridge::Cartridge;
use crate::joypad::Joypad;
use crate::ppu::PPU;

/*
  _______________ $10000  _______________
 | PRG-ROM       |       |               |
 | Upper Bank    |       |               |
 |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
 | PRG-ROM       |       |               |
 | Lower Bank    |       |               |
 |_______________| $8000 |_______________|
 | SRAM          |       | SRAM          |
 |_______________| $6000 |_______________|
 | Expansion ROM |       | Expansion ROM |
 |_______________| $4020 |_______________|
 | I/O Registers |       |               |
 |_ _ _ _ _ _ _ _| $4000 |               |
 | Mirrors       |       | I/O Registers |
 | $2000-$2007   |       |               |
 |_ _ _ _ _ _ _ _| $2008 |               |
 | I/O Registers |       |               |
 |_______________| $2000 |_______________|
 | Mirrors       |       |               |
 | $0000-$07FF   |       |               |
 |_ _ _ _ _ _ _ _| $0800 |               |
 | RAM           |       | RAM           |
 |_ _ _ _ _ _ _ _| $0200 |               |
 | Stack         |       |               |
 |_ _ _ _ _ _ _ _| $0100 |               |
 | Zero Page     |       |               |
 |_______________| $0000 |_______________|
*/

#[allow(dead_code)]
const CPU_RAM_SIZE: usize = 2048;

#[allow(dead_code)]
pub struct Bus<'call> {
    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cart: Cartridge,
    pub ppu: PPU,
    pub joypads: [Joypad; 2],

    gameloop_callback: Box<dyn FnMut(&PPU, &mut [Joypad; 2]) + 'call>,
}

impl Bus<'_> {
    pub fn new<'call>(cart: Cartridge) -> Bus<'call> {
        Bus::new_with_gameloop_callback(cart, move |_ppu: &PPU, _joypads: &mut [Joypad; 2]| {})
    }

    pub fn new_with_gameloop_callback<'call, F>(cart: Cartridge, callback: F) -> Bus<'call>
    where
        F: FnMut(&PPU, &mut [Joypad; 2]) + 'call,
    {
        let ppu = PPU::new(&cart);
        Bus {
            cpu_ram: [0; CPU_RAM_SIZE],
            cart: cart,
            ppu: ppu,
            joypads: [Joypad::new(), Joypad::new()],
            gameloop_callback: Box::from(callback),
        }
    }

    // tick from CPU
    pub fn cpu_tick(&mut self) {
        // TODO more logic

        // tick PPU for 3 times
        let nmi_before = self.has_nmi();
        for _ in 0..3 {
            self.ppu.tick();
        }
        let nmi_after = self.has_nmi();

        if !nmi_before && nmi_after {
            (self.gameloop_callback)(&self.ppu, &mut self.joypads);
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        let v = self.cart.cpu_read(addr);
        if v.is_some() {
            return v.unwrap();
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize],
            // PPU registers mapping
            0x2000..=0x3FFF => self.ppu.cpu_read(addr),
            // TODO APU
            0x4000..=0x4015 => 0,
            // controller register
            0x4016 => self.joypads[0].read(),
            0x4017 => self.joypads[1].read(),
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        let ok = self.cart.cpu_write(addr, value);
        if ok {
            return;
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize] = value,
            0x2000..=0x3FFF => self.ppu.cpu_write(addr, value),
            // TODO DMA register
            0x4014 => (),
            // TODO APU
            0x4000..=0x4013 | 0x4015 => (),
            // controller register
            0x4016 => self.joypads[0].write(value),
            0x4017 => self.joypads[1].write(value),
            _ => (),
        }
    }

    pub fn has_nmi(&self) -> bool {
        self.ppu.has_nmi()
    }

    pub fn reset_nmi(&mut self) {
        self.ppu.reset_nmi();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mem_read_write() {
        let mut bus = Bus::new(Cartridge::new_dummy());
        bus.cpu_write(0x0000, 0xFF);
        assert_eq!(bus.cpu_read(0x0000), 0xFF);
        assert_eq!(bus.cpu_read(0x0800), 0xFF);
        assert_eq!(bus.cpu_read(0x1000), 0xFF);
        assert_eq!(bus.cpu_read(0x1800), 0xFF);
    }
}

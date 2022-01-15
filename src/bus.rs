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

    pub total_system_cycles: u32,

    // DMA
    pub dma_page: u8,
    pub dma_addr: u8,
    pub dma_data: u8,
    // DMA transfers need to be timed accurately. In principle it takes
    // 512 cycles to read and write the 256 bytes of the OAM memory, a
    // read followed by a write. However, the CPU needs to be on an "even"
    // clock cycle, so a dummy cycle of idleness may be required
    pub dma_dummy: bool,
    // Flag to indicate that a DMA transfer is happening
    pub dma_transfer: bool,

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
            total_system_cycles: 0,
            dma_page: 0,
            dma_addr: 0,
            dma_data: 0,
            dma_dummy: true,
            dma_transfer: false,
            gameloop_callback: Box::from(callback),
        }
    }

    // Execute a system tick and return true if CPU should tick
    pub fn system_tick(&mut self) -> bool {
        // The CPU runs 3 times slower than the PPU
        if self.total_system_cycles % 3 == 0 {
            // Is the system performing a DMA transfer form CPU memory to
            // OAM memory on PPU?...
            if self.dma_transfer {
                // ...Yes! We need to wait until the next even CPU clock cycle
                // before it starts...
                if self.dma_dummy {
                    // ...So hang around in here each clock until 1 or 2 cycles
                    // have elapsed...
                    if self.total_system_cycles % 2 == 1 {
                        // ...and finally allow DMA to start
                        self.dma_dummy = false;
                    }
                } else {
                    // DMA can take place!
                    if self.total_system_cycles % 2 == 0 {
                        // On even clock cycles, read from CPU bus
                        self.dma_data =
                            self.cpu_read(((self.dma_page as u16) << 8) | self.dma_addr as u16);
                    } else {
                        // On odd clock cycles, write to PPU OAM
                        self.ppu.oam_data[self.dma_addr as usize] = self.dma_data;
                        // Increment the lo byte of the address
                        self.dma_addr = self.dma_addr.wrapping_add(1);
                        // If this wraps around, we know that 256
                        // bytes have been written, so end the DMA
                        // transfer, and proceed as normal
                        if self.dma_addr == 0x00 {
                            self.dma_transfer = false;
                            self.dma_dummy = true;
                        }
                    }
                }
                self.total_system_cycles = self.total_system_cycles.wrapping_add(1);
                return false;
            } else {
                // No DMA happening, the CPU can tick
                self.total_system_cycles = self.total_system_cycles.wrapping_add(1);
                return true;
            }
        } else {
            self.total_system_cycles = self.total_system_cycles.wrapping_add(1);
            return false;
        }
    }

    pub fn run_gameloop_callback(&mut self) {
        (self.gameloop_callback)(&self.ppu, &mut self.joypads);
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
            0x4014 => {
                // A write to this address initiates a DMA transfer
                self.dma_page = value;
                self.dma_addr = 0x00;
                self.dma_transfer = true;
            }
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

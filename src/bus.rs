use crate::cartridge::Cartridge;

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
const VRAM_SIZE: usize = 65536;

#[allow(dead_code)]
pub struct Bus {
    cpu_ram: [u8; CPU_RAM_SIZE],
    cart_opt: Option<Cartridge>,

    // FIXME before we have PPU and Cartridge,
    //       use an array to represent other parts besides CPU ram
    rest_virt_ram: [u8; VRAM_SIZE],
}

impl Bus {
    #[allow(dead_code)]
    pub fn new() -> Bus {
        Bus {
            cpu_ram: [0; CPU_RAM_SIZE],
            cart_opt: None,
            rest_virt_ram: [0; VRAM_SIZE],
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        if let Some(cart) = &self.cart_opt {
            let (v, ok) = cart.cpu_read(addr);
            if ok {
                return v;
            }
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize],
            _ => self.rest_virt_ram[addr as usize],
        }
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        if let Some(cart) = &self.cart_opt {
            let ok = cart.cpu_write(addr, value);
            if ok {
                return;
            }
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize] = value,
            _ => self.rest_virt_ram[addr as usize] = value,
        }
    }

    pub fn cpu_write_batch(&mut self, start_addr: u16, data: Vec<u8>) {
        for i in 0..data.len() {
            self.cpu_write(start_addr + i as u16, data[i]);
        }
    }

    pub fn insert_cartridge(&mut self, cart: Cartridge) {
        self.cart_opt = Some(cart);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mem_read_write() {
        let mut bus = Bus::new();
        bus.cpu_write(0x0000, 0xFF);
        assert_eq!(bus.cpu_read(0x0000), 0xFF);
        assert_eq!(bus.cpu_read(0x0800), 0xFF);
        assert_eq!(bus.cpu_read(0x1000), 0xFF);
        assert_eq!(bus.cpu_read(0x1800), 0xFF);
    }

    #[test]
    fn test_write_batch() {
        let mut bus = Bus::new();
        bus.cpu_write_batch(0x1000, vec![0x01, 0x02, 0x03, 0x04]);
        // assert_eq!(bus.read(0x1000), 0x01);
        assert_eq!(bus.cpu_read(0x1001), 0x02);
        assert_eq!(bus.cpu_read(0x1002), 0x03);
        assert_eq!(bus.cpu_read(0x1003), 0x04);
    }
}

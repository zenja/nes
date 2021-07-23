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

#[allow(dead_code)]
pub struct Bus {
    cpu_ram: [u8; CPU_RAM_SIZE],
    cart_opt: Option<Cartridge>,
}

impl Bus {
    #[allow(dead_code)]
    pub fn new() -> Bus {
        Bus {
            cpu_ram: [0; CPU_RAM_SIZE],
            cart_opt: None,
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
            _ => 0u8,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        if let Some(ref mut cart) = self.cart_opt {
            let ok = cart.cpu_write(addr, value);
            if ok {
                return;
            }
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize] = value,
            _ => (),
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
}

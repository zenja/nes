#[allow(dead_code)]
const CPU_RAM_SIZE: usize = 2048;
const VRAM_SIZE: usize = 65536;

#[allow(dead_code)]
pub struct Bus {
    cpu_ram: [u8; CPU_RAM_SIZE],

    // FIXME before we have PPU and Cartridge,
    //       use an array to represent other parts besides CPU ram
    other_vram: [u8; VRAM_SIZE],
}

impl Bus {
    #[allow(dead_code)]
    pub fn new() -> Bus {
        Bus {
            cpu_ram: [0; CPU_RAM_SIZE],
            other_vram: [0; VRAM_SIZE],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize],
            _ => self.other_vram[addr as usize],
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0b0000_0111_1111_1111) as usize] = value,
            _ => self.other_vram[addr as usize] = value,
        }
    }

    pub fn write_batch(&mut self, start_addr: u16, data: Vec<u8>) {
        for i in 0..data.len() {
            self.write(start_addr + i as u16, data[i]);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mem_read_write() {
        let mut bus = Bus::new();
        bus.write(0x0000, 0xFF);
        assert_eq!(bus.read(0x0000), 0xFF);
        assert_eq!(bus.read(0x0800), 0xFF);
        assert_eq!(bus.read(0x1000), 0xFF);
        assert_eq!(bus.read(0x1800), 0xFF);
    }

    #[test]
    fn test_write_batch() {
        let mut bus = Bus::new();
        bus.write_batch(0x1000, vec![0x01, 0x02, 0x03, 0x04]);
        // assert_eq!(bus.read(0x1000), 0x01);
        assert_eq!(bus.read(0x1001), 0x02);
        assert_eq!(bus.read(0x1002), 0x03);
        assert_eq!(bus.read(0x1003), 0x04);
    }
}

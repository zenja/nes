#[allow(dead_code)]
const VMEM_SIZE: usize = 65536;
const RAM_START_ADDR: u16 = 0x0000;
const RAM_END_ADDR: u16 = 0x2000;

#[allow(dead_code)]
pub struct Bus {
    mem_mapping: fn(u16) -> u16,
    vmem: [u8; VMEM_SIZE],
}

impl Bus {
    #[allow(dead_code)]
    pub fn new_nes_bus() -> Bus {
        let vmem = [0; VMEM_SIZE];
        Bus {
            mem_mapping: nes_mem_mapping,
            vmem,
        }
    }

    pub fn translate_addr(&self, addr: u16) -> u16 {
        (self.mem_mapping)(addr)
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.vmem[self.translate_addr(addr) as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.vmem[self.translate_addr(addr) as usize] = value
    }

    pub fn write_batch(&mut self, start_addr: u16, data: Vec<u8>) {
        let translated_start_addr = self.translate_addr(start_addr) as usize;
        self.vmem[translated_start_addr..(translated_start_addr + data.len())]
            .copy_from_slice(&data);
    }
}

fn nes_mem_mapping(vmem_addr: u16) -> u16 {
    match vmem_addr {
        RAM_START_ADDR..=RAM_END_ADDR => vmem_addr & 0b0000_0111_1111_1111,
        // TODO more mappings
        _ => vmem_addr,
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mem_read_write() {
        let mut bus = Bus::new_nes_bus();
        bus.write(0x0000, 0xFF);
        assert_eq!(bus.read(0x0000), 0xFF);
        assert_eq!(bus.read(0x0800), 0xFF);
        assert_eq!(bus.read(0x1000), 0xFF);
        assert_eq!(bus.read(0x1800), 0xFF);
    }

    #[test]
    fn test_write_batch() {
        let mut bus = Bus::new_nes_bus();
        bus.write_batch(0x1000, vec![0x01, 0x02, 0x03, 0x04]);
        // assert_eq!(bus.read(0x1000), 0x01);
        assert_eq!(bus.read(0x1001), 0x02);
        assert_eq!(bus.read(0x1002), 0x03);
        assert_eq!(bus.read(0x1003), 0x04);
    }
}

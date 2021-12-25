struct AddrRegister {
    hi: u8,
    lo: u8,
    write_to_hi: bool,
}

impl AddrRegister {
    pub fn new() -> AddrRegister {
        AddrRegister {
            hi: 0,
            lo: 0,
            write_to_hi: true,
        }
    }

    pub fn write(&mut self, value: u8) {
        if self.write_to_hi {
            self.hi = value;
        } else {
            self.lo = value;
        }
        self.write_to_hi = !self.write_to_hi;
    }

    pub fn inc(&mut self, delta: u8) {
        let curr = self.get();
        let new = curr.wrapping_add(delta as u16);
        self.set(new);
    }

    // internal helper to set u16 value directly
    fn set(&mut self, value: u16) {
        self.hi = ((value & 0xff00) >> 8) as u8;
        self.lo = (value & 0x00ff) as u8;
    }

    pub fn get(&self) -> u16 {
        ((self.hi as u16) << 8) | (self.lo as u16)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_write() {
        let mut addr = AddrRegister::new();
        assert_eq!(addr.get(), 0000);

        addr.write(0x12);
        assert_eq!(addr.get(), 0x1200);

        addr.write(0x34);
        assert_eq!(addr.get(), 0x1234);

        addr.write(0x56);
        assert_eq!(addr.get(), 0x5634);

        addr.write(0x78);
        assert_eq!(addr.get(), 0x5678);
    }

    #[test]
    fn test_inc() {
        let mut addr = AddrRegister::new();
        addr.write(0x12);
        addr.write(0x34);
        addr.inc(1);
        assert_eq!(addr.get(), 0x1235);

        addr.write(0xff);
        addr.write(0xff);
        addr.inc(1);
        assert_eq!(addr.get(), 0x0000);

        addr.inc(32);
        assert_eq!(addr.get(), 0x0020);
    }
}

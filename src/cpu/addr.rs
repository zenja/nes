#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Address {
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Immediate(u8),
    Relative(i8),
    Implicit,
    Indirect(u16),
    IndexedIndirect(u8),
    IndirectIndexed(u8),
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum AddrMode {
    Absolute,
    AbsoluteX,
    AbsoluteY,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Immediate,
    Relative,
    Implicit,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
}

impl AddrMode {
    pub fn size(&self) -> u8 {
        match self {
            Self::Absolute => 2,
            Self::AbsoluteX => 2,
            Self::AbsoluteY => 2,
            Self::ZeroPage => 1,
            Self::ZeroPageX => 1,
            Self::ZeroPageY => 1,
            Self::Immediate => 1,
            Self::Relative => 1,
            Self::Implicit => 0,
            Self::Indirect => 2,
            Self::IndexedIndirect => 1,
            Self::IndirectIndexed => 1,
        }
    }

    fn fetch<'a, I>(&self, bytes: I) -> Address
    where
        I: Iterator<Item = &'a u8>,
    {
        let size = self.size();
        let addr = read_bytes(bytes, size);
        match self {
            Self::Absolute => Address::Absolute(addr),
            Self::AbsoluteX => Address::AbsoluteX(addr),
            Self::AbsoluteY => Address::AbsoluteY(addr),
            Self::ZeroPage => Address::ZeroPage(addr as u8),
            Self::ZeroPageX => Address::ZeroPageX(addr as u8),
            Self::ZeroPageY => Address::ZeroPageY(addr as u8),
            Self::Immediate => Address::Immediate(addr as u8),
            Self::Relative => Address::Relative(addr as i8),
            Self::Implicit => Address::Implicit,
            Self::Indirect => Address::Indirect(addr),
            Self::IndexedIndirect => Address::IndexedIndirect(addr as u8),
            Self::IndirectIndexed => Address::IndirectIndexed(addr as u8),
        }
    }
}

fn read_bytes<'a, I>(mut bytes: I, num_bytes: u8) -> u16
where
    I: Iterator<Item = &'a u8>,
{
    match num_bytes {
        0 => 0u16,
        1 => *bytes.next().unwrap() as u16,
        2 => {
            let b0 = bytes.next().unwrap();
            let b1 = bytes.next().unwrap();
            u16::from_le_bytes([*b0, *b1])
        }
        _ => panic!("bytes size not supported: {}", num_bytes),
    }
}

#[cfg(test)]
mod test {
    use itertools::izip;

    use super::*;
    use std::vec;

    #[test]
    fn test_addr_mode_fetch_single_addr() {
        let bytes_list: Vec<Vec<u8>> = vec![
            vec![0xAB, 0xCD],
            vec![0x00, 0x00],
            vec![0x00, 0x00],
            vec![0x00, 0x00],
            vec![0x00, 0x00],
        ];
        let addr_modes: Vec<AddrMode> = vec![
            AddrMode::Absolute,
            AddrMode::Absolute,
            AddrMode::Absolute,
            AddrMode::Absolute,
            AddrMode::Absolute,
        ];
        let expected_addrs: Vec<Address> = vec![
            Address::Absolute(0xCDAB),
            Address::Absolute(0x0000),
            Address::Absolute(0x0000),
            Address::Absolute(0x0000),
            Address::Absolute(0x0000),
        ];
        for (bytes, addr_mode, expected_addr) in izip!(bytes_list, addr_modes, expected_addrs) {
            let actual_addr = addr_mode.fetch(bytes.iter());
            assert_eq!(actual_addr, expected_addr);
        }
    }

    #[test]
    fn test_addr_mode_fetch_multiple_addr() {
        let bytes = vec![0xA0, 0xB0, 0xC0, 0xD0, 0x12, 0x34, 0x56];
        let mut iter = bytes.iter();
        assert_eq!(
            AddrMode::Absolute.fetch(&mut iter),
            Address::Absolute(0xB0A0)
        );
        assert_eq!(
            AddrMode::AbsoluteX.fetch(&mut iter),
            Address::AbsoluteX(0xD0C0)
        );
        assert_eq!(
            AddrMode::Immediate.fetch(&mut iter),
            Address::Immediate(0x12)
        );
        assert_eq!(
            AddrMode::Indirect.fetch(&mut iter),
            Address::Indirect(0x5634)
        );
    }
}

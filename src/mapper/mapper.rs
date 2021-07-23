pub trait Mapper {
    fn cpu_read_mapping(&self, addr: u16) -> (u16, bool);
    fn cpu_write_mapping(&self, addr: u16) -> (u16, bool);
}

impl core::fmt::Debug for dyn Mapper {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn new(mapper_id: u8, num_prg_banks: u8, num_chr_banks: u8) -> Option<Box<dyn Mapper>> {
    use super::mapper_0::Mapper0;
    match mapper_id {
        0 => Some(Box::new(Mapper0::new(num_prg_banks, num_chr_banks))),
        _ => None,
    }
}

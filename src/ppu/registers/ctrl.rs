use core::panic;

use bitflags::bitflags;

bitflags! {
   // 7  bit  0
   // ---- ----
   // VPHB SINN
   // |||| ||||
   // |||| ||++- Base nametable address
   // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
   // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
   // |||| |     (0: add 1, going across; 1: add 32, going down)
   // |||| +---- Sprite pattern table address for 8x8 sprites
   // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
   // |||+------ Background pattern table address (0: $0000; 1: $1000)
   // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
   // |+-------- PPU master/slave select
   // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
   // +--------- Generate an NMI at the start of the
   //            vertical blanking interval (0: off; 1: on)
   pub struct CtrlRegister: u8 {
       const NAMETABLE1              = 0b00000001;
       const NAMETABLE2              = 0b00000010;
       const VRAM_ADDR_INCREMENT     = 0b00000100;
       const SPRITE_PATTERN_ADDR     = 0b00001000;
       const BACKROUND_PATTERN_ADDR  = 0b00010000;
       const SPRITE_SIZE             = 0b00100000;
       const MASTER_SLAVE_SELECT     = 0b01000000;
       const GENERATE_NMI            = 0b10000000;
   }
}

impl CtrlRegister {
    pub fn new() -> CtrlRegister {
        CtrlRegister::from_bits_truncate(0)
    }

    pub fn write(&mut self, value: u8) {
        self.bits = value;
    }

    pub fn get_base_nametable_addr(&self) -> u16 {
        match self.bits & 0b00000011 {
            0b00 => 0x2000,
            0b01 => 0x2400,
            0b10 => 0x2800,
            0b11 => 0x2C00,
            _ => panic!("impossible!"),
        }
    }

    pub fn get_vram_addr_inc(&self) -> u8 {
        if self.contains(CtrlRegister::VRAM_ADDR_INCREMENT) {
            32
        } else {
            1
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_vram_addr_inc() {
        let mut ctrl = CtrlRegister::VRAM_ADDR_INCREMENT;
        assert_eq!(ctrl.get_vram_addr_inc(), 32);

        ctrl.remove(CtrlRegister::VRAM_ADDR_INCREMENT);
        assert_eq!(ctrl.get_vram_addr_inc(), 1);
    }

    #[test]
    fn test_get_base_nametable_addr() {
        let mut ctrl = CtrlRegister::empty();
        assert_eq!(ctrl.get_base_nametable_addr(), 0x2000);

        ctrl.insert(CtrlRegister::NAMETABLE1);
        assert_eq!(ctrl.get_base_nametable_addr(), 0x2400);

        ctrl.insert(CtrlRegister::NAMETABLE2);
        assert_eq!(ctrl.get_base_nametable_addr(), 0x2C00);

        ctrl.remove(CtrlRegister::NAMETABLE1);
        assert_eq!(ctrl.get_base_nametable_addr(), 0x2800);
    }
}

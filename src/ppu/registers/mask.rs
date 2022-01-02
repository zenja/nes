use bitflags::bitflags;

bitflags! {
    // 7  bit  0
    // ---- ----
    // BGRs bMmG
    // |||| ||||
    // |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
    // |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    // |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    // |||| +---- 1: Show background
    // |||+------ 1: Show sprites
    // ||+------- Emphasize red
    // |+-------- Emphasize green
    // +--------- Emphasize blue
    pub struct MaskRegister: u8 {
        const GREYSCALE                 = 0b00000001;
        const SHOW_LEFTMOST_BACKGROUND  = 0b00000010;
        const SHOW_LEFTMOST_SPRITE      = 0b00000100;
        const SHOW_BACKGROUND           = 0b00001000;
        const SHOW_SPRITES              = 0b00010000;
        const EMPHASIZE_RED             = 0b00100000;
        const EMPHASIZE_GREEN           = 0b01000000;
        const EMPHASIZE_BLUE            = 0b10000000;
    }
}

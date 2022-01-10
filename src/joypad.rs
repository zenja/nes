use bitflags::bitflags;

bitflags! {
    // Ref: https://wiki.nesdev.org/w/index.php/Controller_reading_code
    pub struct JoypadStatus: u8 {
        const RIGHT             = 0b10000000;
        const LEFT              = 0b01000000;
        const DOWN              = 0b00100000;
        const UP                = 0b00010000;
        const START             = 0b00001000;
        const SELECT            = 0b00000100;
        const BUTTON_B          = 0b00000010;
        const BUTTON_A          = 0b00000001;
    }
}

pub struct Joypad {
    // strobe bit on - controller reports only status of the button A on every read
    // strobe bit off - controller cycles through all buttons
    strobe: bool,
    next_btn_idx: u8,
    status: JoypadStatus,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            next_btn_idx: 0,
            status: JoypadStatus::from_bits_truncate(0),
        }
    }

    pub fn write(&mut self, value: u8) {
        // first bit indicates strobe mode on/off
        self.strobe = (value & 1) == 1;
        if self.strobe {
            self.next_btn_idx = 0;
        }
    }

    pub fn read(&mut self) -> u8 {
        fn is_btn_on(status: &JoypadStatus, btn_idx: u8) -> bool {
            (status.bits & (1 << btn_idx)) > 0
        }

        if self.next_btn_idx > 7 {
            return 1;
        }
        let response: u8 = if is_btn_on(&self.status, self.next_btn_idx) {
            1
        } else {
            0
        };
        if !self.strobe && self.next_btn_idx <= 7 {
            self.next_btn_idx += 1;
        }
        response
    }

    pub fn set(&mut self, status: &JoypadStatus) {
        self.status.set(*status, true);
    }

    pub fn unset(&mut self, status: &JoypadStatus) {
        self.status.set(*status, false);
    }
}

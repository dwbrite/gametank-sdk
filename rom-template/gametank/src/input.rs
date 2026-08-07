use bit_field::BitField;

const GPR1: *const u8 = 0x2008 as *const u8;
const GPR2: *const u8 = 0x2009 as *const u8;

#[inline(always)]
fn read_gpr1() -> u8 {
    unsafe { core::ptr::read_volatile(GPR1) }
}

#[inline(always)]
fn read_gpr2() -> u8 {
    unsafe { core::ptr::read_volatile(GPR2) }
}

#[derive(Debug, Copy, Clone)]
pub enum Buttons {
    Start,
    A,
    B,
    C,
    Up,
    Down,
    Left,
    Right
}

impl Buttons {
    /// good shit, const brained
    const fn idx(&self) -> usize {
        match self {
            Buttons::Start => 7,
            Buttons::A => 6,
            Buttons::B => 4,
            Buttons::C => 5,

            Buttons::Up => 3,
            Buttons::Down => 2,
            Buttons::Left => 1,
            Buttons::Right => 0,
        }
    }
}

/// Controller for a specific port.
///
/// The port is a const generic, so `read()` is monomorphized at compile time
/// with no runtime branching.
///
/// - `Controller<1>` for port 1
/// - `Controller<2>` for port 2
pub struct GenesisGamepad<const PORT: u8> {
    pub buttons: u8,
    pub buttons_last: u8,
}

impl<const PORT: u8> GenesisGamepad<PORT> {
    pub const fn new() -> Self {
        Self {
            buttons: 0,
            buttons_last: 0,
        }
    }
}

impl GenesisGamepad<1> {
    /// Read port 1 controller state.
    #[inline(always)]
    pub fn read(&mut self) {
        // Reset select by reading GPR2, then read GPR1 twice
        let _ = read_gpr2();
        let byte0 = read_gpr1();
        let byte1 = read_gpr1();

        self.buttons_last = self.buttons;
        // bits: start, a | c, b, up, down, left, right
        self.buttons = ((!byte0 << 2) & 0b1100_0000) | (!byte1 & 0b0011_1111);
    }
}

impl GenesisGamepad<2> {
    /// Read port 2 controller state.
    #[inline(always)]
    pub fn read(&mut self) {
        // Reset select by reading GPR1, then read GPR2 twice
        let _ = read_gpr1();
        let byte0 = read_gpr2();
        let byte1 = read_gpr2();

        self.buttons_last = self.buttons;
        // bits: start, a | c, b, up, down, left, right
        self.buttons = ((!byte0 << 2) & 0b1100_0000) | (!byte1 & 0b0011_1111);
    }
}

impl<const PORT: u8> GenesisGamepad<PORT> {
    #[inline]
    pub fn is_pressed(&self, button: Buttons) -> bool {
        self.buttons.get_bit(button.idx())
    }

    #[inline]
    pub fn was_pressed(&self, button: Buttons) -> bool {
        self.buttons_last.get_bit(button.idx())
    }

    /// Returns true only on the frame the button was first pressed (edge-trigger).
    #[inline]
    pub fn just_pressed(&self, button: Buttons) -> bool {
        self.is_pressed(button) && !self.was_pressed(button)
    }

    /// Returns true only on the frame the button was released (edge-trigger).
    #[inline]
    pub fn just_released(&self, button: Buttons) -> bool {
        !self.is_pressed(button) && self.was_pressed(button)
    }
}

use heapless::Vec;
use usbd_human_interface_device::page::Keyboard;

pub enum Key {
    None,
    Std(Keyboard),
    Shifted(Keyboard),
    NeverShifted(Keyboard),
    Layout(u8),
    HR((Keyboard, Keyboard)),
}

// pub to_keys(key: Key) {

// }

pub struct Modifiers {
    pub Alt: (bool, usize),
    pub Ctrl: (bool, usize),
    pub Gui: (bool, usize),
    pub Shift: (bool, usize),
}
impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            Alt: (false, 0),
            Ctrl: (false, 0),
            Gui: (false, 0),
            Shift: (false, 0),
        }
    }

    pub fn is_active(&self, index: usize) -> bool {
        (self.Alt.0 && self.Alt.1 == index)
            || (self.Ctrl.0 && self.Ctrl.1 == index)
            || (self.Gui.0 && self.Gui.1 == index)
            || (self.Shift.0 && self.Shift.1 == index)
    }
}

#[repr(u16)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum KC {
    None = 0,

    A = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    F = 6,
    G = 7,
    H = 8,
    I = 9,
    J = 10,
    K = 11,
    L = 12,
    M = 13,
    N = 14,
    O = 15,
    P = 16,
    Q = 17,
    R = 18,
    S = 19,
    T = 20,
    U = 21,
    V = 22,
    W = 23,
    X = 24,
    Y = 25,
    Z = 26,

    Fr_e_acute = 50,

    Num0 = 300,
    Num1 = 301,
    Num2 = 302,
    Num3 = 303,
    Num4 = 304,
    Num5 = 305,
    Num6 = 306,
    Num7 = 307,
    Num8 = 308,
    Num9 = 309,

    Minus = 400,
    Equal = 401,
    LeftBracket = 402,
    RightBracket = 403,
    Backslash = 404,
    NonusHash = 405,
    SemiColon = 406,
    Quote = 407,
    Grave = 408,
    Comma = 409,
    Dot = 410,
    Slash = 411,
    NonusBackslash = 412,

    Tilde = 500,
    Exclaim = 501,
    At = 502,
    Hash = 503,
    Dollar = 504,
    Percentage = 505,
    Circumflex = 506,
    Ampersand = 507,
    Asterix = 508,
    LeftParent = 509,
    RightParent = 510,
    Underscore = 511,
    Plus = 512,
    LeftCurly = 513,
    RightCurly = 514,
    Pipe = 515,
    Colon = 516,
    DoubleQuote = 517,
    LowerThan = 518,
    GreaterThan = 519,
    Question = 520,

    ALT = 1000,
    CTRL = 1001,
    GUI = 1002,
    SHIFT = 1003,

    HomeAltA = 2000,
    HomeAltU = 2001,
    HomeGuiS = 2002,
    HomeGuiI = 2003,
    HomeCtrlE = 2004,
    HomeCtrlT = 2005,
    HomeSftN = 2006,
    HomeSftR = 2007,

    // Home(KC) = 2000,
    LAY(u8) = 10000,
}

fn push(array: &mut [Keyboard; 5], val: Keyboard) {
    if let Some(index) = array.iter().position(|c| *c == Keyboard::NoEventIndicated) {
        array[index] = val;
        // true
        // } else {
        //     false
    }
}

impl KC {
    #[rustfmt::skip]
    pub fn to_usb_code(&self, modifiers: &Modifiers) -> [Keyboard;5] {
        let mut output = [Keyboard::NoEventIndicated; 5];

        if modifiers.Alt.0 || *self == KC::ALT {
            push(&mut output,Keyboard::LeftAlt);
        }
        if modifiers.Ctrl.0 || *self == KC::CTRL {
            push(&mut output, Keyboard::LeftControl);
        }
        if modifiers.Gui.0 || *self == KC::GUI {
            push(&mut output, Keyboard::LeftGUI);
        }

        // Exclude numbers and symbols from shift
        if (modifiers.Shift.0 || *self == KC::SHIFT) && (*self < KC::Num0 || *self > KC::Question)
        {
            push(&mut output, Keyboard::LeftShift);
        }

        match *self {
            KC::A => push(&mut output, Keyboard::A),
            KC::B => push(&mut output, Keyboard::B),
            KC::C => push(&mut output, Keyboard::C),
            KC::D => push(&mut output, Keyboard::D),
            KC::E => push(&mut output, Keyboard::E),
            KC::F => push(&mut output, Keyboard::F),
            KC::G => push(&mut output, Keyboard::G),
            KC::H => push(&mut output, Keyboard::H),
            KC::I => push(&mut output, Keyboard::I),
            KC::J => push(&mut output, Keyboard::J),
            KC::K => push(&mut output, Keyboard::K),
            KC::L => push(&mut output, Keyboard::L),
            KC::M => push(&mut output, Keyboard::M),
            KC::N => push(&mut output, Keyboard::N),
            KC::O => push(&mut output, Keyboard::O),
            KC::P => push(&mut output, Keyboard::P),
            KC::Q => push(&mut output, Keyboard::Q),
            KC::R => push(&mut output, Keyboard::R),
            KC::S => push(&mut output, Keyboard::S),
            KC::T => push(&mut output, Keyboard::T),
            KC::U => push(&mut output, Keyboard::U),
            KC::V => push(&mut output, Keyboard::V),
            KC::W => push(&mut output, Keyboard::W),
            KC::X => push(&mut output, Keyboard::X),
            KC::Y => push(&mut output, Keyboard::Y),
            KC::Z => push(&mut output, Keyboard::Z),

            KC::Num0 => push(&mut output, Keyboard::Keyboard0),
            KC::Num1 => push(&mut output, Keyboard::Keyboard1),
            KC::Num2 => push(&mut output, Keyboard::Keyboard2),
            KC::Num3 => push(&mut output, Keyboard::Keyboard3),
            KC::Num4 => push(&mut output, Keyboard::Keyboard4),
            KC::Num5 => push(&mut output, Keyboard::Keyboard5),
            KC::Num6 => push(&mut output, Keyboard::Keyboard6),
            KC::Num7 => push(&mut output, Keyboard::Keyboard7),
            KC::Num8 => push(&mut output, Keyboard::Keyboard8),
            KC::Num9 => push(&mut output, Keyboard::Keyboard9),

            KC::Minus          => push(&mut output, Keyboard::Minus),
            KC::Equal          => push(&mut output, Keyboard::Equal),
            KC::LeftBracket    => push(&mut output, Keyboard::LeftBrace),
            KC::RightBracket   => push(&mut output, Keyboard::RightBrace),
            KC::Backslash      => push(&mut output, Keyboard::Backslash),
            KC::NonusHash      => push(&mut output, Keyboard::NonUSHash),
            KC::SemiColon      => push(&mut output, Keyboard::Semicolon),
            KC::Quote          => push(&mut output, Keyboard::Apostrophe),
            KC::Grave          => push(&mut output, Keyboard::Grave),
            KC::Comma          => push(&mut output, Keyboard::Comma),
            KC::Dot            => push(&mut output, Keyboard::Dot),
            KC::Slash          => push(&mut output, Keyboard::ForwardSlash),
            KC::NonusBackslash => push(&mut output, Keyboard::NonUSBackslash),

            KC::Tilde          => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Grave); }
            KC::Exclaim        => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard1); }
            KC::At             => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard2); }
            KC::Hash           => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard3); }
            KC::Dollar         => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard4); }
            KC::Percentage     => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard5); }
            KC::Circumflex     => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard6); }
            KC::Ampersand      => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard7); }
            KC::Asterix        => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard8); }
            KC::LeftParent     => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard9); }
            KC::RightParent    => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Keyboard0); }
            KC::Underscore     => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Minus); }
            KC::Plus           => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Equal); }
            KC::LeftCurly      => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::LeftBrace); }
            KC::RightCurly     => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::RightBrace); }
            KC::Pipe           => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Backslash); }
            KC::Colon          => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Semicolon); }
            KC::DoubleQuote    => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Apostrophe); }
            KC::LowerThan      => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Comma); }
            KC::GreaterThan    => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::Dot); }
            KC::Question       => { push(&mut output, Keyboard::LeftShift); push(&mut output, Keyboard::ForwardSlash); }

            KC::HomeAltA       => push(&mut output, Keyboard::A),
            KC::HomeAltU       => push(&mut output, Keyboard::U),
            KC::HomeGuiS       => push(&mut output, Keyboard::S),
            KC::HomeGuiI       => push(&mut output, Keyboard::I),
            KC::HomeCtrlE      => push(&mut output, Keyboard::E),
            KC::HomeCtrlT      => push(&mut output, Keyboard::T),
            KC::HomeSftN     => push(&mut output, Keyboard::N),
            KC::HomeSftR     => push(&mut output, Keyboard::R),

            _ => {}
        }

        output
    }
}

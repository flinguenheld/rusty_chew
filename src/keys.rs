use heapless::Vec;
use usbd_human_interface_device::page::Keyboard;

use crate::utils::options::BUFFER_LENGTH;

pub struct Modifiers {
    pub alt: (bool, usize),
    pub alt_gr: (bool, usize),
    pub ctrl: (bool, usize),
    pub gui: (bool, usize),
    pub shift: (bool, usize),
}
impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            alt: (false, 0),
            alt_gr: (false, 0),
            ctrl: (false, 0),
            gui: (false, 0),
            shift: (false, 0),
        }
    }

    pub fn is_active(&self, index: usize) -> bool {
        (self.alt.0 && self.alt.1 == index)
            || (self.alt_gr.0 && self.alt_gr.1 == index)
            || (self.ctrl.0 && self.ctrl.1 == index)
            || (self.gui.0 && self.gui.1 == index)
            || (self.shift.0 && self.shift.1 == index)
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

    E_acute = 50,

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

    // Macros - No held
    E_circu = 600,

    ALT = 10000,
    ALTGR = 10001,
    CTRL = 10002,
    GUI = 10003,
    SHIFT = 10004,

    HomeAltA = 20000,
    HomeAltU = 20001,
    HomeGuiS = 20002,
    HomeGuiI = 20003,
    HomeCtrlE = 20004,
    HomeCtrlT = 20005,
    HomeSftN = 20006,
    HomeSftR = 20007,

    // Home(KC) = 2000,
    LAY(u8) = 60000,
}

fn push_to_buffer(combination: [Keyboard; 6], buffer: &mut Vec<[Keyboard; 6], BUFFER_LENGTH>) {
    buffer.push(combination).ok();
}

fn push(mut array: [Keyboard; 6], val: Keyboard) -> [Keyboard; 6] {
    if let Some(index) = array.iter().position(|c| *c == Keyboard::NoEventIndicated) {
        array[index] = val;
    }
    array
}

impl KC {
    #[rustfmt::skip]
    pub fn to_usb_code(&self, modifiers: &Modifiers, buffer: &mut Vec<[Keyboard; 6], 20>) {
        let mut output = [Keyboard::NoEventIndicated; 6];

        if modifiers.alt.0 || *self == KC::ALT {
            output = push(output, Keyboard::LeftAlt);
        }
        // TODO Is alt + alt gr possible ?
        if modifiers.alt_gr.0 || *self == KC::ALTGR {
            output = push(output, Keyboard::RightAlt);
        }
        if modifiers.ctrl.0 || *self == KC::CTRL {
            output = push(output, Keyboard::LeftControl);
        }
        if modifiers.gui.0 || *self == KC::GUI {
            output = push(output, Keyboard::LeftGUI);
        }

        // Exclude numbers and symbols from shift
        if (modifiers.shift.0 || *self == KC::SHIFT) && (*self < KC::Num0 || *self > KC::Question) {
            output = push(output, Keyboard::LeftShift);
        }

        match *self {
            KC::A => push_to_buffer(push(output, Keyboard::A), buffer),
            KC::B => push_to_buffer(push(output, Keyboard::B), buffer),
            KC::C => push_to_buffer(push(output, Keyboard::C), buffer),
            KC::D => push_to_buffer(push(output, Keyboard::D), buffer),
            KC::E => push_to_buffer(push(output, Keyboard::E), buffer),
            KC::F => push_to_buffer(push(output, Keyboard::F), buffer),
            KC::G => push_to_buffer(push(output, Keyboard::G), buffer),
            KC::H => push_to_buffer(push(output, Keyboard::H), buffer),
            KC::I => push_to_buffer(push(output, Keyboard::I), buffer),
            KC::J => push_to_buffer(push(output, Keyboard::J), buffer),
            KC::K => push_to_buffer(push(output, Keyboard::K), buffer),
            KC::L => push_to_buffer(push(output, Keyboard::L), buffer),
            KC::M => push_to_buffer(push(output, Keyboard::M), buffer),
            KC::N => push_to_buffer(push(output, Keyboard::N), buffer),
            KC::O => push_to_buffer(push(output, Keyboard::O), buffer),
            KC::P => push_to_buffer(push(output, Keyboard::P), buffer),
            KC::Q => push_to_buffer(push(output, Keyboard::Q), buffer),
            KC::R => push_to_buffer(push(output, Keyboard::R), buffer),
            KC::S => push_to_buffer(push(output, Keyboard::S), buffer),
            KC::T => push_to_buffer(push(output, Keyboard::T), buffer),
            KC::U => push_to_buffer(push(output, Keyboard::U), buffer),
            KC::V => push_to_buffer(push(output, Keyboard::V), buffer),
            KC::W => push_to_buffer(push(output, Keyboard::W), buffer),
            KC::X => push_to_buffer(push(output, Keyboard::X), buffer),
            KC::Y => push_to_buffer(push(output, Keyboard::Y), buffer),
            KC::Z => push_to_buffer(push(output, Keyboard::Z), buffer),

            KC::E_acute =>  push_to_buffer(push(push(output, Keyboard::RightAlt), Keyboard::E), buffer) ,
            // KC::E_circu => { push_to_buffer(push(push(output.clone(), Keyboard::RightAlt), Keyboard::Grave), buffer);
            //                  push_to_buffer(push( output, Keyboard::E), buffer); }
            KC::E_circu => { push_to_buffer(push(push(output.clone(), Keyboard::A), Keyboard::B), buffer);
                             push_to_buffer(push( output, Keyboard::C), buffer); }

            KC::Num0 => push_to_buffer(push(output, Keyboard::Keyboard0), buffer),
            KC::Num1 => push_to_buffer(push(output, Keyboard::Keyboard1), buffer),
            KC::Num2 => push_to_buffer(push(output, Keyboard::Keyboard2), buffer),
            KC::Num3 => push_to_buffer(push(output, Keyboard::Keyboard3), buffer),
            KC::Num4 => push_to_buffer(push(output, Keyboard::Keyboard4), buffer),
            KC::Num5 => push_to_buffer(push(output, Keyboard::Keyboard5), buffer),
            KC::Num6 => push_to_buffer(push(output, Keyboard::Keyboard6), buffer),
            KC::Num7 => push_to_buffer(push(output, Keyboard::Keyboard7), buffer),
            KC::Num8 => push_to_buffer(push(output, Keyboard::Keyboard8), buffer),
            KC::Num9 => push_to_buffer(push(output, Keyboard::Keyboard9), buffer),

            KC::Minus => push_to_buffer(push(output, Keyboard::Minus), buffer),
            KC::Equal => push_to_buffer(push(output, Keyboard::Equal), buffer),
            KC::LeftBracket => push_to_buffer(push(output, Keyboard::LeftBrace), buffer),
            KC::RightBracket => push_to_buffer(push(output, Keyboard::RightBrace), buffer),
            KC::Backslash => push_to_buffer(push(output, Keyboard::Backslash), buffer),
            KC::NonusHash => push_to_buffer(push(output, Keyboard::NonUSHash), buffer),
            KC::SemiColon => push_to_buffer(push(output, Keyboard::Semicolon), buffer),
            KC::Quote => push_to_buffer(push(output, Keyboard::Apostrophe), buffer),
            KC::Grave => push_to_buffer(push(output, Keyboard::Grave), buffer),
            KC::Comma => push_to_buffer(push(output, Keyboard::Comma), buffer),
            KC::Dot => push_to_buffer(push(output, Keyboard::Dot), buffer),
            KC::Slash => push_to_buffer(push(output, Keyboard::ForwardSlash), buffer),
            KC::NonusBackslash => {
                push_to_buffer(push(output, Keyboard::NonUSBackslash), buffer)
            }

            KC::Tilde => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Grave), buffer, ); }
            KC::Exclaim => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard1), buffer, ); }
            KC::At => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard2), buffer, ); }
            KC::Hash => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard3), buffer, ); }
            KC::Dollar => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard4), buffer, ); }
            KC::Percentage => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard5), buffer, ); }
            KC::Circumflex => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard6), buffer, ); }
            KC::Ampersand => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard7), buffer, ); }
            KC::Asterix => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard8), buffer, ); }
            KC::LeftParent => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard9), buffer, ); }
            KC::RightParent => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Keyboard0), buffer, ); }
            KC::Underscore => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Minus), buffer, ); }
            KC::Plus => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Equal), buffer, ); }
            KC::LeftCurly => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::LeftBrace), buffer, ); }
            KC::RightCurly => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::RightBrace), buffer, ); }
            KC::Pipe => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Backslash), buffer, ); }
            KC::Colon => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Semicolon), buffer, ); }
            KC::DoubleQuote => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Apostrophe), buffer, ); }
            KC::LowerThan => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Comma), buffer, ); }
            KC::GreaterThan => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::Dot), buffer, ); }
            KC::Question => { push_to_buffer( push(push(output, Keyboard::LeftShift), Keyboard::ForwardSlash), buffer, ); }

            KC::HomeAltA => push_to_buffer(push(output, Keyboard::A), buffer),
            KC::HomeAltU => push_to_buffer(push(output, Keyboard::U), buffer),
            KC::HomeGuiS => push_to_buffer(push(output, Keyboard::S), buffer),
            KC::HomeGuiI => push_to_buffer(push(output, Keyboard::I), buffer),
            KC::HomeCtrlE => push_to_buffer(push(output, Keyboard::E), buffer),
            KC::HomeCtrlT => push_to_buffer(push(output, Keyboard::T), buffer),
            KC::HomeSftN => push_to_buffer(push(output, Keyboard::N), buffer),
            KC::HomeSftR => push_to_buffer(push(output, Keyboard::R), buffer),

            _ => {}
        }
    }
}

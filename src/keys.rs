use alloc::vec::Vec;
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

    Num0 = 40,
    Num1 = 41,
    Num2 = 42,
    Num3 = 43,
    Num4 = 44,
    Num5 = 45,
    Num6 = 46,
    Num7 = 47,
    Num8 = 48,
    Num9 = 49,

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

    LAY(u8) = 10000,
}

pub fn add_rename(alt: bool, ctrl: bool, gui: bool, shift: bool, key: KC) -> Vec<Keyboard> {
    let mut output = Vec::new();

    if alt || key == KC::ALT {
        output.push(Keyboard::LeftAlt);
    }
    if ctrl || key == KC::CTRL {
        output.push(Keyboard::LeftControl);
    }
    if gui || key == KC::GUI {
        output.push(Keyboard::LeftGUI);
    }

    if (shift || key == KC::SHIFT) && (key < KC::Minus || key > KC::Question) {
        output.push(Keyboard::LeftShift);
    }

    match key {
        KC::A => output.push(Keyboard::A),
        KC::B => output.push(Keyboard::B),
        KC::C => output.push(Keyboard::C),
        KC::D => output.push(Keyboard::D),
        KC::E => output.push(Keyboard::E),
        KC::F => output.push(Keyboard::F),
        KC::G => output.push(Keyboard::G),
        KC::H => output.push(Keyboard::H),
        KC::I => output.push(Keyboard::I),
        KC::J => output.push(Keyboard::J),
        KC::K => output.push(Keyboard::K),
        KC::L => output.push(Keyboard::L),
        KC::M => output.push(Keyboard::M),
        KC::N => output.push(Keyboard::N),
        KC::O => output.push(Keyboard::O),
        KC::P => output.push(Keyboard::P),
        KC::Q => output.push(Keyboard::Q),
        KC::R => output.push(Keyboard::R),
        KC::S => output.push(Keyboard::S),
        KC::T => output.push(Keyboard::T),
        KC::U => output.push(Keyboard::U),
        KC::V => output.push(Keyboard::V),
        KC::W => output.push(Keyboard::W),
        KC::X => output.push(Keyboard::X),
        KC::Y => output.push(Keyboard::Y),
        KC::Z => output.push(Keyboard::Z),

        KC::Num0 => output.push(Keyboard::Keyboard0),
        KC::Num1 => output.push(Keyboard::Keyboard1),
        KC::Num2 => output.push(Keyboard::Keyboard2),
        KC::Num3 => output.push(Keyboard::Keyboard3),
        KC::Num4 => output.push(Keyboard::Keyboard4),
        KC::Num5 => output.push(Keyboard::Keyboard5),
        KC::Num6 => output.push(Keyboard::Keyboard6),
        KC::Num7 => output.push(Keyboard::Keyboard7),
        KC::Num8 => output.push(Keyboard::Keyboard8),
        KC::Num9 => output.push(Keyboard::Keyboard9),

        KC::Minus => output.push(Keyboard::Minus),
        KC::Equal => output.push(Keyboard::Equal),
        KC::LeftBracket => output.push(Keyboard::LeftBrace),
        KC::RightBracket => output.push(Keyboard::RightBrace),
        KC::Backslash => output.push(Keyboard::Backslash),
        KC::NonusHash => output.push(Keyboard::NonUSHash),
        KC::SemiColon => output.push(Keyboard::Semicolon),
        KC::Quote => output.push(Keyboard::Apostrophe),
        KC::Grave => output.push(Keyboard::Grave),
        KC::Comma => output.push(Keyboard::Comma),
        KC::Dot => output.push(Keyboard::Dot),
        KC::Slash => output.push(Keyboard::ForwardSlash),
        KC::NonusBackslash => output.push(Keyboard::NonUSBackslash),

        KC::Tilde => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Grave);
        }
        KC::Exclaim => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard1);
        }
        KC::At => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard2);
        }
        KC::Hash => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard3);
        }
        KC::Dollar => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard4);
        }
        KC::Percentage => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard5);
        }
        KC::Circumflex => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard6);
        }
        KC::Ampersand => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard7);
        }
        KC::Asterix => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard8);
        }
        KC::LeftParent => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard9);
        }
        KC::RightParent => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Keyboard0);
        }
        KC::Underscore => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Minus);
        }
        KC::Plus => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Equal);
        }
        KC::LeftCurly => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::LeftBrace);
        }
        KC::RightCurly => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::RightBrace);
        }
        KC::Pipe => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Backslash);
        }
        KC::Colon => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Semicolon);
        }
        KC::DoubleQuote => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Apostrophe);
        }
        KC::LowerThan => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Comma);
        }
        KC::GreaterThan => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::Dot);
        }
        KC::Question => {
            output.push(Keyboard::LeftShift);
            output.push(Keyboard::ForwardSlash);
        }

        _ => {}
    }

    output
}

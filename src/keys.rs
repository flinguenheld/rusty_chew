use heapless::Deque;
use usbd_human_interface_device::page::Keyboard;

use crate::utils::options::BUFFER_LENGTH;

const DEAD_CIRCUMFLEX: [Keyboard; 6] = [
    Keyboard::RightAlt,
    Keyboard::Keyboard6,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
];
const DEAD_DIAERIS: [Keyboard; 6] = [
    Keyboard::LeftShift,
    Keyboard::RightAlt,
    Keyboard::Apostrophe,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
];
const DEAD_GRAVE: [Keyboard; 6] = [
    Keyboard::RightAlt,
    Keyboard::Grave,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
    Keyboard::NoEventIndicated,
];
const EMPTY: [Keyboard; 6] = [Keyboard::NoEventIndicated; 6];

/// Keep the state and the matrix index for each modifier.
pub struct Modifiers {
    pub alt: (bool, usize),
    pub alt_gr: (bool, usize),
    pub ctrl: (bool, usize),
    pub gui: (bool, usize),
    pub shift: (bool, usize),
}

#[rustfmt::skip]
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

    pub fn nb_on(&self) -> usize {
        let mut nb = 0;
        if self.alt.0 { nb += 1 }
        if self.alt_gr.0 { nb += 1 }
        if self.ctrl.0 { nb += 1 }
        if self.gui.0 { nb += 1 }
        if self.shift.0 { nb += 1 }
        nb
    }
}

/// Allow to memorise whish key is currently used and if another key has validated the dead action.
pub struct DeadLayout {
    pub active: bool,
    pub done: bool,
    pub index: usize,
}

impl DeadLayout {
    pub fn new(active: bool, index: usize) -> DeadLayout {
        DeadLayout {
            active,
            index,
            done: false,
        }
    }
}

#[rustfmt::skip]
#[allow(dead_code)]
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

    EAcute = 50,

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
    ACircum = 600,
    AGrave = 601,
    ADiaer = 602,
        ECircum = 603,
        EGrave = 604,
        EDiaer = 605,
    ICircum = 606,
    IGrave = 607,
    IDiaer = 608,
        OCircum = 609,
        OGrave = 610,
        ODiaer = 611,
    UCircum = 612,
    UGrave = 613,
    UDiaer = 614,
        YCircum = 615,
        YGrave = 616,
        YDiaer = 617,

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
    Layout(usize) = 60000,
    LayDead(usize) = 60001,
}

#[rustfmt::skip]
impl KC {
    fn new_combination(&self, modifiers: &Modifiers) -> [Keyboard; 6] {
        let mut output = EMPTY;

        // Exclude numbers and symbols from shift
        if (modifiers.shift.0 || *self == KC::SHIFT) && (*self < KC::Num0 || *self > KC::Question) { output[0] = Keyboard::LeftShift; }
        if modifiers.alt.0 || *self == KC::ALT {                                                     output[1] = Keyboard::LeftAlt; }
        if modifiers.alt_gr.0 || *self == KC::ALTGR {                                                output[2] = Keyboard::RightAlt; }
        if modifiers.ctrl.0 || *self == KC::CTRL {                                                   output[3] = Keyboard::LeftControl; }
        if modifiers.gui.0 || *self == KC::GUI {                                                     output[4] = Keyboard::LeftGUI; }

        output
    }

    #[rustfmt::skip]
    pub fn to_usb_code(&self, modifiers: &Modifiers, mut buffer: Deque<[Keyboard; 6], BUFFER_LENGTH>) -> Deque<[Keyboard; 6], BUFFER_LENGTH> {

        let mut output = self.new_combination(modifiers);

        match *self {
            KC::None => { buffer.push_back([Keyboard::NoEventIndicated; 6]).ok(); },

            KC::A => { output[5] = Keyboard::A; buffer.push_back(output).ok(); },
            KC::B => { output[5] = Keyboard::B; buffer.push_back(output).ok(); },
            KC::C => { output[5] = Keyboard::C; buffer.push_back(output).ok(); },
            KC::D => { output[5] = Keyboard::D; buffer.push_back(output).ok(); },
            KC::E => { output[5] = Keyboard::E; buffer.push_back(output).ok(); },
            KC::F => { output[5] = Keyboard::F; buffer.push_back(output).ok(); },
            KC::G => { output[5] = Keyboard::G; buffer.push_back(output).ok(); },
            KC::H => { output[5] = Keyboard::H; buffer.push_back(output).ok(); },
            KC::I => { output[5] = Keyboard::I; buffer.push_back(output).ok(); },
            KC::J => { output[5] = Keyboard::J; buffer.push_back(output).ok(); },
            KC::K => { output[5] = Keyboard::K; buffer.push_back(output).ok(); },
            KC::L => { output[5] = Keyboard::L; buffer.push_back(output).ok(); },
            KC::M => { output[5] = Keyboard::M; buffer.push_back(output).ok(); },
            KC::N => { output[5] = Keyboard::N; buffer.push_back(output).ok(); },
            KC::O => { output[5] = Keyboard::O; buffer.push_back(output).ok(); },
            KC::P => { output[5] = Keyboard::P; buffer.push_back(output).ok(); },
            KC::Q => { output[5] = Keyboard::Q; buffer.push_back(output).ok(); },
            KC::R => { output[5] = Keyboard::R; buffer.push_back(output).ok(); },
            KC::S => { output[5] = Keyboard::S; buffer.push_back(output).ok(); },
            KC::T => { output[5] = Keyboard::T; buffer.push_back(output).ok(); },
            KC::U => { output[5] = Keyboard::U; buffer.push_back(output).ok(); },
            KC::V => { output[5] = Keyboard::V; buffer.push_back(output).ok(); },
            KC::W => { output[5] = Keyboard::W; buffer.push_back(output).ok(); },
            KC::X => { output[5] = Keyboard::X; buffer.push_back(output).ok(); },
            KC::Y => { output[5] = Keyboard::Y; buffer.push_back(output).ok(); },
            KC::Z => { output[5] = Keyboard::Z; buffer.push_back(output).ok(); },

            KC::EAcute => { output[2] = Keyboard::RightAlt; output[5] = Keyboard::E; buffer.push_back(output).ok(); },

            KC::ACircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::A; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::ADiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::A; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::AGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::A; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::ECircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::E; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::EDiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::E; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::EGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::E; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::ICircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::I; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::IDiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::I; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::IGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::I; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::OCircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::O; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::ODiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::O; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::OGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::O; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::UCircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::U; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::UDiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::U; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::UGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::U; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::YCircum => { buffer.push_back(DEAD_CIRCUMFLEX).ok(); output[5] = Keyboard::Y; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::YDiaer  => { buffer.push_back(DEAD_DIAERIS).ok();    output[5] = Keyboard::Y; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },
            KC::YGrave  => { buffer.push_back(DEAD_GRAVE).ok();      output[5] = Keyboard::Y; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); },

            KC::Num0 => { output[5] = Keyboard::Keyboard0; buffer.push_back(output).ok(); }
            KC::Num1 => { output[5] = Keyboard::Keyboard1; buffer.push_back(output).ok(); }
            KC::Num2 => { output[5] = Keyboard::Keyboard2; buffer.push_back(output).ok(); }
            KC::Num3 => { output[5] = Keyboard::Keyboard3; buffer.push_back(output).ok(); }
            KC::Num4 => { output[5] = Keyboard::Keyboard4; buffer.push_back(output).ok(); }
            KC::Num5 => { output[5] = Keyboard::Keyboard5; buffer.push_back(output).ok(); }
            KC::Num6 => { output[5] = Keyboard::Keyboard6; buffer.push_back(output).ok(); }
            KC::Num7 => { output[5] = Keyboard::Keyboard7; buffer.push_back(output).ok(); }
            KC::Num8 => { output[5] = Keyboard::Keyboard8; buffer.push_back(output).ok(); }
            KC::Num9 => { output[5] = Keyboard::Keyboard9; buffer.push_back(output).ok(); }

            KC::Minus          => { output[5] = Keyboard::Minus;          buffer.push_back(output).ok(); }
            KC::Equal          => { output[5] = Keyboard::Equal;          buffer.push_back(output).ok(); }
            KC::LeftBracket    => { output[5] = Keyboard::LeftBrace;      buffer.push_back(output).ok(); }
            KC::RightBracket   => { output[5] = Keyboard::RightBrace;     buffer.push_back(output).ok(); }
            KC::Backslash      => { output[5] = Keyboard::Backslash;      buffer.push_back(output).ok(); }
            KC::NonusHash      => { output[5] = Keyboard::NonUSHash;      buffer.push_back(output).ok(); }
            KC::SemiColon      => { output[5] = Keyboard::Semicolon;      buffer.push_back(output).ok(); }
            KC::Quote          => { output[5] = Keyboard::Apostrophe;     buffer.push_back(output).ok(); }
            KC::Grave          => { output[5] = Keyboard::Grave;          buffer.push_back(output).ok(); }
            KC::Comma          => { output[5] = Keyboard::Comma;          buffer.push_back(output).ok(); }
            KC::Dot            => { output[5] = Keyboard::Dot;            buffer.push_back(output).ok(); }
            KC::Slash          => { output[5] = Keyboard::ForwardSlash;   buffer.push_back(output).ok(); }
            KC::NonusBackslash => { output[5] = Keyboard::NonUSBackslash; buffer.push_back(output).ok(); }

            KC::Tilde       => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Grave;        buffer.push_back(output).ok(); }
            KC::Exclaim     => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard1;    buffer.push_back(output).ok(); }
            KC::At          => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard2;    buffer.push_back(output).ok(); }
            KC::Hash        => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard3;    buffer.push_back(output).ok(); }
            KC::Dollar      => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard4;    buffer.push_back(output).ok(); }
            KC::Percentage  => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard5;    buffer.push_back(output).ok(); }
            KC::Circumflex  => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard6;    buffer.push_back(output).ok(); }
            KC::Ampersand   => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard7;    buffer.push_back(output).ok(); }
            KC::Asterix     => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard8;    buffer.push_back(output).ok(); }
            KC::LeftParent  => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard9;    buffer.push_back(output).ok(); }
            KC::RightParent => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Keyboard0;    buffer.push_back(output).ok(); }
            KC::Underscore  => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Minus;        buffer.push_back(output).ok(); }
            KC::Plus        => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Equal;        buffer.push_back(output).ok(); }
            KC::LeftCurly   => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::LeftBrace;    buffer.push_back(output).ok(); }
            KC::RightCurly  => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::RightBrace;   buffer.push_back(output).ok(); }
            KC::Pipe        => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Backslash;    buffer.push_back(output).ok(); }
            KC::Colon       => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Semicolon;    buffer.push_back(output).ok(); }
            KC::DoubleQuote => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Apostrophe;   buffer.push_back(output).ok(); }
            KC::LowerThan   => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Comma;        buffer.push_back(output).ok(); }
            KC::GreaterThan => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Dot;          buffer.push_back(output).ok(); }
            KC::Question    => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::ForwardSlash; buffer.push_back(output).ok(); }

            KC::HomeAltA =>  { output[5] = Keyboard::A; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeAltU =>  { output[5] = Keyboard::U; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeGuiS =>  { output[5] = Keyboard::S; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeGuiI =>  { output[5] = Keyboard::I; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeCtrlE => { output[5] = Keyboard::E; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeCtrlT => { output[5] = Keyboard::T; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeSftN =>  { output[5] = Keyboard::N; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeSftR =>  { output[5] = Keyboard::R; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }

            _ => {}
        }

        buffer
    }
}

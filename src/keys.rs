#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::utils::{ modifiers::Modifiers,
           options::{BUFFER_CASE_LENGTH, BUFFER_LENGTH, TEMPO_DEAD_KEY, TEMPO_MACRO},
};
use heapless::{Deque, Vec};
use usbd_human_interface_device::{device::mouse::WheelMouseReport, page::Keyboard};

// LEADER KEY ---------------------------------------------------------------------------
pub const LEADER_KEY_COMBINATIONS: [([KC; 3], KC); 3] = [
    ([KC::F, KC::L, KC::None], KC::MacroGit),
    ([KC::M, KC::HomeAltA, KC::None], KC::MacroMail),
    ([KC::M, KC::HomeGuiS, KC::None], KC::MacroMailShort),
];

// COMBOS -------------------------------------------------------------------------------
pub const COMBOS: [([KC; 2], KC); 2] = [
    ([KC::Layout(1), KC::Layout(2)], KC::Layout(3)),
    ([KC::P, KC::O], KC::At),
];

// --------------------------------------------------------------------------------------
const DEAD_CIRCUMFLEX: [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Keyboard6];
const DEAD_DIAERIS:    [Keyboard; 3] = [Keyboard::LeftShift, Keyboard::RightAlt, Keyboard::Apostrophe, ];
const DEAD_GRAVE:      [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Grave];

// --------------------------------------------------------------------------------------
// Help KC conversion along.
// This buffer is fill here to be then empty by the writing report.
// Each entry is a vec of Keyboard pages followed by a tempo (a break can be mandatory e.q with dead keys).

#[derive(PartialEq, Default)]
pub struct BuffCase {
    pub key_code: Vec<Keyboard, BUFFER_CASE_LENGTH>,
    pub matrix_indexes: Vec<usize, BUFFER_CASE_LENGTH>,
    pub tempo: u32,
}

pub struct Buffer {
    pub keys: Deque<BuffCase, BUFFER_LENGTH>,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer { keys: Deque::new() }
    }

    // Add a new entry in the Deque with active mods (without excluded), the keys and set the tempo.
    fn add(mut self, keys: &[Keyboard], indexes: &[usize], mods: &Modifiers, excluded_mods: &[Keyboard], tempo: u32) -> Self {
        let mut key_code = Vec::new();
        key_code.extend(mods.active()
                         .iter()
                         .filter(|m| !excluded_mods.contains(m))
                         .copied());
        key_code.extend(keys.iter().copied());

        let mut matrix_indexes = Vec::new();
        matrix_indexes.extend(indexes.iter().copied());

        self.keys.push_back(BuffCase {key_code, matrix_indexes, tempo}).ok();
        self
    }

    fn add_simple(self, keys: &[Keyboard], indexes: &[usize], mods: &Modifiers) -> Self {
        self.add(keys, indexes, mods, &[], 0)
    }

    fn add_no_mods(self, keys: &[Keyboard], indexes: &[usize], tempo: u32) -> Self {
        self.add(keys, indexes, &Modifiers::new(), &[], tempo)
    }

    pub fn close(self) -> Self {
        self.add_no_mods(&[Keyboard::NoEventIndicated], &[], 0)
    }
}

// --------------------------------------------------------------------------------------

#[rustfmt::skip]
#[allow(dead_code)]
#[repr(u16)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum KC {
    None = 0,

    A = 10,
    B = 11,
    C = 12,
    D = 13,
    E = 14,
    F = 15,
    G = 16,
    H = 17,
    I = 18,
    J = 19,
    K = 20,
    L = 21,
    M = 22,
    N = 23,
    O = 24,
    P = 25,
    Q = 26,
    R = 27,
    S = 28,
    T = 29,
    U = 30,
    V = 31,
    W = 32,
    X = 33,
    Y = 34,
    Z = 35,

    CCedilla = 100,
    EAcute = 101,
    AE = 102,
    OE = 103,

    Enter = 500,
    Space = 501,
    Esc = 502,
    Del = 503,
    BackSpace = 504,
    Tab = 505,
    STab = 506,
    Home = 507,
    End = 508,
    PageUp = 509,
    PageDown = 510,

    Left = 600,
    Down = 601,
    Up = 602,
    Right = 603,

    // Num & symbols - No shift
    Num0 = 800,
    Num1 = 801,
    Num2 = 802,
    Num3 = 803,
    Num4 = 804,
    Num5 = 805,
    Num6 = 806,
    Num7 = 807,
    Num8 = 808,
    Num9 = 809,

    Minus = 1000,
    Equal = 1001,
    LeftBracket = 1002,
    RightBracket = 1003,
    Backslash = 1004,
    NonusHash = 1005,
    SemiColon = 1006,
    Quote = 1007,
    Grave = 1008,
    Comma = 1009,
    Dot = 1010,
    Slash = 1011,
    NonusBackslash = 1012,

    Tilde = 2000,
    Exclaim = 2001,
    At = 2002,
    Hash = 2003,
    Dollar = 2004,
    Percentage = 2005,
    Circumflex = 2006,
    Ampersand = 2007,
    Asterix = 2008,
    LeftParent = 2009,
    RightParent = 2010,
    Underscore = 2011,
    Plus = 2012,
    LeftCurly = 2013,
    RightCurly = 2014,
    Pipe = 2015,
    Colon = 2016,
    DoubleQuote = 2017,
    LowerThan = 2018,
    GreaterThan = 2019,
    Question = 2020,

        GuillemetL = 3021,
        GuillemetD = 3022,
        Diameter = 3023,
        Degre = 3024,
        Euro = 3025,
        Pound = 3026,
        Yen = 3027,

    // Macros - No held
    ACircum = 4000,
    AGrave = 4001,
    ADiaer = 4002,
        ECircum = 4003,
        EGrave = 4004,
        EDiaer = 4005,
    ICircum = 4006,
    IGrave = 4007,
    IDiaer = 4008,
        OCircum = 4009,
        OGrave = 4010,
        ODiaer = 4011,
    UCircum = 4012,
    UGrave = 4013,
    UDiaer = 4014,
        YCircum = 4015,
        YGrave = 4016,
        YDiaer = 4017,

    Alt = 10000,
    Altgr = 10001,
    Ctrl = 10002,
    Gui = 10003,
    Shift = 10004,

    HomeAltA = 20000,
    HomeAltU = 20001,
    HomeGuiS = 20002,
    HomeGuiI = 20003,
    HomeCtrlE = 20004,
    HomeCtrlT = 20005,
    HomeSftN = 20006,
    HomeSftR = 20007,

        DeadCircumflex = 30000,
        DeadDiaeris = 30001,
        DeadGrave = 30002,
    
    MouseBtLeft = 50000,
    MouseBtMiddle = 50001,
    MouseBtRight = 50002,
        MouseLeft = 50020,
        MouseDown = 50021,
        MouseUp = 50022,
        MouseRight = 50023,
    MouseWheelLeft = 50030,
    MouseWheelDown = 50031,
    MouseWheelUp = 50032,
    MouseWheelRight = 50033,
        MouseSpeed1 = 50040,
        MouseSpeed2 = 50041,
        MouseSpeed3 = 50042,
        MouseSpeed4 = 50043,

    Layout(usize) = 60000,
    LayDead(usize) = 60001,

        LeaderKey = 61000,

    MacroGit = 62001,
    MacroMail = 62002,
    MacroMailShort = 62003,
}

impl KC {
    // Mouse ----------------------------------------------------------------------------
    pub fn usb_mouse_move(&self, mut report: WheelMouseReport, speed: i8) -> WheelMouseReport {
        match *self {
            KC::MouseLeft  => report.x = i8::saturating_add(report.x, -speed),
            KC::MouseDown  => report.y = i8::saturating_add(report.y, speed),
            KC::MouseUp    => report.y = i8::saturating_add(report.y, -speed),
            KC::MouseRight => report.x = i8::saturating_add(report.x, speed),

            KC::MouseWheelLeft  => report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel, speed),
            KC::MouseWheelDown  => report.vertical_wheel   = i8::saturating_add(report.vertical_wheel, -speed),
            KC::MouseWheelUp    => report.vertical_wheel   = i8::saturating_add(report.vertical_wheel, speed),
            KC::MouseWheelRight => report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel, -speed),
            _ => {}
        }
        report
    }

    // Keyboard -------------------------------------------------------------------------
    /// Convert a Chew keycode into an array of Keyboard page.
    pub fn usb_code(&self, buffer: Buffer, indexes: &[usize], mods: &Modifiers) -> Buffer {
        match *self {
            KC::None => buffer.add_simple(&[Keyboard::NoEventIndicated], &[], mods),

            KC::A => buffer.add_simple(&[Keyboard::A], indexes, mods),
            KC::B => buffer.add_simple(&[Keyboard::B], indexes, mods),
            KC::C => buffer.add_simple(&[Keyboard::C], indexes, mods),
            KC::D => buffer.add_simple(&[Keyboard::D], indexes, mods),
            KC::E => buffer.add_simple(&[Keyboard::E], indexes, mods),
            KC::F => buffer.add_simple(&[Keyboard::F], indexes, mods),
            KC::G => buffer.add_simple(&[Keyboard::G], indexes, mods),
            KC::H => buffer.add_simple(&[Keyboard::H], indexes, mods),
            KC::I => buffer.add_simple(&[Keyboard::I], indexes, mods),
            KC::J => buffer.add_simple(&[Keyboard::J], indexes, mods),
            KC::K => buffer.add_simple(&[Keyboard::K], indexes, mods),
            KC::L => buffer.add_simple(&[Keyboard::L], indexes, mods),
            KC::M => buffer.add_simple(&[Keyboard::M], indexes, mods),
            KC::N => buffer.add_simple(&[Keyboard::N], indexes, mods),
            KC::O => buffer.add_simple(&[Keyboard::O], indexes, mods),
            KC::P => buffer.add_simple(&[Keyboard::P], indexes, mods),
            KC::Q => buffer.add_simple(&[Keyboard::Q], indexes, mods),
            KC::R => buffer.add_simple(&[Keyboard::R], indexes, mods),
            KC::S => buffer.add_simple(&[Keyboard::S], indexes, mods),
            KC::T => buffer.add_simple(&[Keyboard::T], indexes, mods),
            KC::U => buffer.add_simple(&[Keyboard::U], indexes, mods),
            KC::V => buffer.add_simple(&[Keyboard::V], indexes, mods),
            KC::W => buffer.add_simple(&[Keyboard::W], indexes, mods),
            KC::X => buffer.add_simple(&[Keyboard::X], indexes, mods),
            KC::Y => buffer.add_simple(&[Keyboard::Y], indexes, mods),
            KC::Z => buffer.add_simple(&[Keyboard::Z], indexes, mods),

            KC::CCedilla => buffer.add(&[Keyboard::RightAlt, Keyboard::Comma], indexes, mods, &[Keyboard::RightAlt], 0),
            KC::EAcute   => buffer.add(&[Keyboard::RightAlt, Keyboard::E],     indexes, mods, &[Keyboard::RightAlt], 0),
            KC::AE       => buffer.add(&[Keyboard::RightAlt, Keyboard::Z],     indexes, mods, &[Keyboard::RightAlt], 0),
            KC::OE       => buffer.add(&[Keyboard::RightAlt, Keyboard::K],     indexes, mods, &[Keyboard::RightAlt], 0),

            KC::Enter     => buffer.add_simple(&[Keyboard::ReturnEnter],       indexes, mods),
            KC::Space     => buffer.add_simple(&[Keyboard::Space],             indexes, mods),
            KC::Esc       => buffer.add_simple(&[Keyboard::Escape],            indexes, mods),
            KC::Del       => buffer.add_simple(&[Keyboard::DeleteBackspace],   indexes, mods),
            KC::BackSpace => buffer.add_simple(&[Keyboard::DeleteForward],     indexes, mods),
            KC::Tab       => buffer.add_simple(&[Keyboard::Tab],               indexes, mods),
            KC::STab      => buffer.add(&[Keyboard::LeftShift, Keyboard::Tab], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Home      => buffer.add_simple(&[Keyboard::Home],              indexes, mods),
            KC::End       => buffer.add_simple(&[Keyboard::End],               indexes, mods),
            KC::PageUp    => buffer.add_simple(&[Keyboard::PageUp],            indexes, mods),
            KC::PageDown  => buffer.add_simple(&[Keyboard::PageDown],          indexes, mods),

            KC::Left      => buffer.add_simple(&[Keyboard::LeftArrow],         indexes, mods),
            KC::Down      => buffer.add_simple(&[Keyboard::DownArrow],         indexes, mods),
            KC::Up        => buffer.add_simple(&[Keyboard::UpArrow],           indexes, mods),
            KC::Right     => buffer.add_simple(&[Keyboard::RightArrow],        indexes, mods),

            KC::ACircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], indexes, mods),
            KC::ADiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], indexes, mods),
            KC::AGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], indexes, mods),

            KC::ECircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], indexes, mods),
            KC::EDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], indexes, mods),
            KC::EGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], indexes, mods),

            KC::ICircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], indexes, mods),
            KC::IDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], indexes, mods),
            KC::IGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], indexes, mods),

            KC::OCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], indexes, mods),
            KC::ODiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], indexes, mods),
            KC::OGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], indexes, mods),

            KC::UCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], indexes, mods),
            KC::UDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], indexes, mods),
            KC::UGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], indexes, mods),

            KC::YCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], indexes, mods),
            KC::YDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], indexes, mods),
            KC::YGrave  => buffer.add_no_mods(&DEAD_GRAVE,      &[], TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], indexes, mods),

            KC::Num0 => buffer.add(&[Keyboard::Keyboard0], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num1 => buffer.add(&[Keyboard::Keyboard1], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num2 => buffer.add(&[Keyboard::Keyboard2], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num3 => buffer.add(&[Keyboard::Keyboard3], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num4 => buffer.add(&[Keyboard::Keyboard4], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num5 => buffer.add(&[Keyboard::Keyboard5], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num6 => buffer.add(&[Keyboard::Keyboard6], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num7 => buffer.add(&[Keyboard::Keyboard7], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num8 => buffer.add(&[Keyboard::Keyboard8], indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Num9 => buffer.add(&[Keyboard::Keyboard9], indexes, mods, &[Keyboard::LeftShift], 0),

            KC::Minus          => buffer.add_simple(&[Keyboard::Minus],          indexes, mods),
            KC::Equal          => buffer.add_simple(&[Keyboard::Equal],          indexes, mods),
            KC::LeftBracket    => buffer.add_simple(&[Keyboard::LeftBrace],      indexes, mods),
            KC::RightBracket   => buffer.add_simple(&[Keyboard::RightBrace],     indexes, mods),
            KC::Backslash      => buffer.add_simple(&[Keyboard::Backslash],      indexes, mods),
            KC::NonusHash      => buffer.add_simple(&[Keyboard::NonUSHash],      indexes, mods),
            KC::SemiColon      => buffer.add_simple(&[Keyboard::Semicolon],      indexes, mods),
            KC::Quote          => buffer.add_simple(&[Keyboard::Apostrophe],     indexes, mods),
            KC::Grave          => buffer.add_simple(&[Keyboard::Grave],          indexes, mods),
            KC::Comma          => buffer.add_simple(&[Keyboard::Comma],          indexes, mods),
            KC::Dot            => buffer.add_simple(&[Keyboard::Dot],            indexes, mods),
            KC::Slash          => buffer.add_simple(&[Keyboard::ForwardSlash],   indexes, mods),
            KC::NonusBackslash => buffer.add_simple(&[Keyboard::NonUSBackslash], indexes, mods),

            KC::Tilde       => buffer.add(&[Keyboard::LeftShift, Keyboard::Grave],        indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Exclaim     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard1],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::At          => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard2],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Hash        => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard3],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Dollar      => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard4],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Percentage  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard5],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Circumflex  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard6],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Ampersand   => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard7],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Asterix     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard8],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::LeftParent  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard9],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::RightParent => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard0],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Underscore  => buffer.add(&[Keyboard::LeftShift, Keyboard::Minus],        indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Plus        => buffer.add(&[Keyboard::LeftShift, Keyboard::Equal],        indexes, mods, &[Keyboard::LeftShift], 0),
            KC::LeftCurly   => buffer.add(&[Keyboard::LeftShift, Keyboard::LeftBrace],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::RightCurly  => buffer.add(&[Keyboard::LeftShift, Keyboard::RightBrace],   indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Pipe        => buffer.add(&[Keyboard::LeftShift, Keyboard::Backslash],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Colon       => buffer.add(&[Keyboard::LeftShift, Keyboard::Semicolon],    indexes, mods, &[Keyboard::LeftShift], 0),
            KC::DoubleQuote => buffer.add(&[Keyboard::LeftShift, Keyboard::Apostrophe],   indexes, mods, &[Keyboard::LeftShift], 0),
            KC::LowerThan   => buffer.add(&[Keyboard::LeftShift, Keyboard::Comma],        indexes, mods, &[Keyboard::LeftShift], 0),
            KC::GreaterThan => buffer.add(&[Keyboard::LeftShift, Keyboard::Dot],          indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Question    => buffer.add(&[Keyboard::LeftShift, Keyboard::ForwardSlash], indexes, mods, &[Keyboard::LeftShift], 0),

            KC::GuillemetL  => buffer.add(&[Keyboard::RightAlt,  Keyboard::LeftBrace],    indexes, mods, &[Keyboard::RightAlt],  0),
            KC::GuillemetD  => buffer.add(&[Keyboard::RightAlt,  Keyboard::RightBrace],   indexes, mods, &[Keyboard::RightAlt],  0),
            KC::Diameter    => buffer.add(&[Keyboard::RightAlt,  Keyboard::L],            indexes, mods, &[Keyboard::RightAlt],  0),
            KC::Degre       => buffer.add(&[Keyboard::LeftShift, Keyboard::RightAlt],     indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Euro        => buffer.add(&[Keyboard::RightAlt,  Keyboard::Keyboard5],    indexes, mods, &[Keyboard::RightAlt],  0),
            KC::Pound       => buffer.add(&[Keyboard::LeftShift, Keyboard::RightAlt],     indexes, mods, &[Keyboard::LeftShift], 0),
            KC::Yen         => buffer.add(&[Keyboard::RightAlt,  Keyboard::Minus],        indexes, mods, &[Keyboard::RightAlt],  0),

            KC::Alt       => buffer.add_no_mods(&[Keyboard::LeftAlt],     &[], 0),
            KC::Altgr     => buffer.add_no_mods(&[Keyboard::RightAlt],    &[], 0),
            KC::Ctrl      => buffer.add_no_mods(&[Keyboard::LeftControl], &[], 0),
            KC::Gui       => buffer.add_no_mods(&[Keyboard::LeftGUI],     &[], 0),
            KC::Shift     => buffer.add_no_mods(&[Keyboard::LeftShift],   &[], 0),

            KC::HomeAltA  => buffer.add_simple(&[Keyboard::A], indexes, mods),
            KC::HomeAltU  => buffer.add_simple(&[Keyboard::U], indexes, mods),
            KC::HomeGuiS  => buffer.add_simple(&[Keyboard::S], indexes, mods),
            KC::HomeGuiI  => buffer.add_simple(&[Keyboard::I], indexes, mods),
            KC::HomeCtrlE => buffer.add_simple(&[Keyboard::E], indexes, mods),
            KC::HomeCtrlT => buffer.add_simple(&[Keyboard::T], indexes, mods),
            KC::HomeSftN  => buffer.add_simple(&[Keyboard::N], indexes, mods),
            KC::HomeSftR  => buffer.add_simple(&[Keyboard::R], indexes, mods),

            // // --
            KC::MacroGit => {
                buffer.add_no_mods(&[Keyboard::F],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::I],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::G],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::U],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::H],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::D],                              &[], TEMPO_MACRO)
            }
            KC::MacroMail => {
                buffer.add_no_mods(&[Keyboard::F],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::O],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::R],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::T],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::LeftShift, Keyboard::Keyboard2], &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::I],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::G],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::U],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::H],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::D],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::Dot],                            &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::F],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::R],                              &[], TEMPO_MACRO)
            }
            KC::MacroMailShort => {
                buffer.add_no_mods(&[Keyboard::F],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::LeftShift, Keyboard::Keyboard2], &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::I],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::G],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::U],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::N],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::H],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::E],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::L],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::D],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::Dot],                            &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::F],                              &[], TEMPO_MACRO)
                      .add_no_mods(&[Keyboard::R],                              &[], TEMPO_MACRO)
            }
            _ => buffer,
        }
    }
}

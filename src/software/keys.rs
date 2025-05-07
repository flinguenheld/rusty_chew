#![cfg_attr(rustfmt, rustfmt_skip)]
use heapless::{Deque, Vec};
use usbd_human_interface_device::page::Keyboard;

use crate::options::{BUFFER_CASE_LENGTH, BUFFER_LENGTH, TEMPO_DEAD_KEY};
use super::{macros::str_to_usb_code, modifiers::Modifiers};

// --------------------------------------------------------------------------------------
const DEAD_CIRCUMFLEX: [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Keyboard6];
const DEAD_DIAERIS:    [Keyboard; 3] = [Keyboard::LeftShift, Keyboard::RightAlt, Keyboard::Apostrophe, ];
const DEAD_GRAVE:      [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Grave];

// --------------------------------------------------------------------------------------
// Help KC conversion along.
// This buffer is filled here to be then emptied by the writing report.
// Each entry is a vec of Keyboard pages followed by a tempo (a break can be mandatory e.q with dead keys).
#[derive(PartialEq, Default)]
pub struct BuffCase {
    pub key_code: Vec<Keyboard, BUFFER_CASE_LENGTH>,
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
    fn add(mut self, keys: &[Keyboard], mods: &Modifiers, excluded_mods: &[Keyboard], tempo: u32) -> Self {
        let mut key_code = Vec::new();
        key_code.extend(mods.active()
                         .iter()
                         .filter(|m| !excluded_mods.contains(m))
                         .copied());
        key_code.extend(keys.iter().copied());

        self.keys.push_back(BuffCase {key_code, tempo}).ok();
        self
    }

    fn add_simple(self, keys: &[Keyboard], mods: &Modifiers) -> Self {
        self.add(keys, mods, &[], 0)
    }

    fn add_no_mods(self, keys: &[Keyboard], tempo: u32) -> Self {
        self.add(keys, &Modifiers::new(), &[], tempo)
    }
}

// --------------------------------------------------------------------------------------

#[rustfmt::skip]
#[allow(dead_code)]
#[repr(u16)]
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum KC {
    None = 0,
    Done = 1,
    DoneButKeep = 2,

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

        Copyright = 3027,
        Registered = 3028,

        ExpOne = 3029,
        ExpTwo = 3030,
        ExpThree = 3031,
        Pilcrow = 3032,
        Multi = 3033,
        Div = 3034,
        Beta = 3035,

        Quarter = 3036,
        Half = 3037,
        ThreeQuarter = 3038,
         
        Yen = 3039, // Keep this one last, it is use by chew as a limit

    F1  = 4000,
    F2  = 4001,
    F3  = 4002,
    F4  = 4003,
    F5  = 4004,
    F6  = 4005,
    F7  = 4006,
    F8  = 4007,
    F9  = 4008,
    F10 = 4009,
    F11 = 4010,
    F12 = 4011,
    F13 = 4012,
    F14 = 4013,
    F15 = 4014,
    F16 = 4015,
    F17 = 4016,
    F18 = 4017,
    F19 = 4018,
    F20 = 4019,
    F21 = 4020,
    F22 = 4021,
    F23 = 4022,
    F24 = 4023,

    // Macros - No held
    ACircum = 5000,
    AGrave = 5001,
    ADiaer = 5002,
        ECircum = 5003,
        EGrave = 5004,
        EDiaer = 5005,
    ICircum = 5006,
    IGrave = 5007,
    IDiaer = 5008,
        OCircum = 5009,
        OGrave = 5010,
        ODiaer = 5011,
    UCircum = 5012,
    UGrave = 5013,
    UDiaer = 5014,
        YCircum = 5015,
        YGrave = 5016,
        YDiaer = 5017,

    Qu = 5050,
    Tion = 5051,
        

    Alt = 10000,
    Altgr = 10001,
    Ctl = 10002,
    Gui = 10003,
    Sft = 10004,

    HomeRow(&'static KC, &'static KC) = 11000,

        DeadCircumflex = 20000,
        DeadDiaeris = 20001,
        DeadGrave = 20002,
    
    MouseBtLeft = 30000,
    MouseBtMiddle = 30001,
    MouseBtRight = 30002,
        MouseLeft = 30020,
        MouseDown = 30021,
        MouseUp = 30022,
        MouseRight = 30023,
    MouseWheelLeft = 30030,
    MouseWheelDown = 30031,
    MouseWheelUp = 30032,
    MouseWheelRight = 30033,
        MouseSpeed1 = 30040,
        MouseSpeed2 = 30041,
        MouseSpeed3 = 30042,
        MouseSpeed4 = 30043,

    Layout(usize) = 40000,
    LaySet(usize) = 40001,
    LayDead(usize) = 40002,

        LeaderKey = 51000,
        CapLock = 51001,
        DynMacRecord = 51002,
        DynMacGo = 51003,

    MacroGit = 52001,
    MacroMail = 52002,
    MacroMailShort = 52003,
    MacroHTTPS = 52004,
    MacroDotfiles = 52005,
    MacroNixOS = 52006,
}

impl KC {
    /// Convert a Chew keycode into an array of Keyboard page.
    pub fn usb_code(&self, buffer: Buffer, mods: &Modifiers) -> Buffer {
        match *self {
            KC::None => buffer.add_simple(&[Keyboard::NoEventIndicated], mods),

            KC::A => buffer.add_simple(&[Keyboard::A], mods),
            KC::B => buffer.add_simple(&[Keyboard::B], mods),
            KC::C => buffer.add_simple(&[Keyboard::C], mods),
            KC::D => buffer.add_simple(&[Keyboard::D], mods),
            KC::E => buffer.add_simple(&[Keyboard::E], mods),
            KC::F => buffer.add_simple(&[Keyboard::F], mods),
            KC::G => buffer.add_simple(&[Keyboard::G], mods),
            KC::H => buffer.add_simple(&[Keyboard::H], mods),
            KC::I => buffer.add_simple(&[Keyboard::I], mods),
            KC::J => buffer.add_simple(&[Keyboard::J], mods),
            KC::K => buffer.add_simple(&[Keyboard::K], mods),
            KC::L => buffer.add_simple(&[Keyboard::L], mods),
            KC::M => buffer.add_simple(&[Keyboard::M], mods),
            KC::N => buffer.add_simple(&[Keyboard::N], mods),
            KC::O => buffer.add_simple(&[Keyboard::O], mods),
            KC::P => buffer.add_simple(&[Keyboard::P], mods),
            KC::Q => buffer.add_simple(&[Keyboard::Q], mods),
            KC::R => buffer.add_simple(&[Keyboard::R], mods),
            KC::S => buffer.add_simple(&[Keyboard::S], mods),
            KC::T => buffer.add_simple(&[Keyboard::T], mods),
            KC::U => buffer.add_simple(&[Keyboard::U], mods),
            KC::V => buffer.add_simple(&[Keyboard::V], mods),
            KC::W => buffer.add_simple(&[Keyboard::W], mods),
            KC::X => buffer.add_simple(&[Keyboard::X], mods),
            KC::Y => buffer.add_simple(&[Keyboard::Y], mods),
            KC::Z => buffer.add_simple(&[Keyboard::Z], mods),

            KC::CCedilla => buffer.add(&[Keyboard::RightAlt, Keyboard::Comma], mods, &[Keyboard::RightAlt], 0),
            KC::EAcute   => buffer.add(&[Keyboard::RightAlt, Keyboard::E],     mods, &[Keyboard::RightAlt], 0),
            KC::AE       => buffer.add(&[Keyboard::RightAlt, Keyboard::Z],     mods, &[Keyboard::RightAlt], 0),
            KC::OE       => buffer.add(&[Keyboard::RightAlt, Keyboard::K],     mods, &[Keyboard::RightAlt], 0),

            KC::Enter     => buffer.add_simple(&[Keyboard::ReturnEnter],       mods),
            KC::Space     => buffer.add_simple(&[Keyboard::Space],             mods),
            KC::Esc       => buffer.add_simple(&[Keyboard::Escape],            mods),
            KC::Del       => buffer.add_simple(&[Keyboard::DeleteBackspace],   mods),
            KC::BackSpace => buffer.add_simple(&[Keyboard::DeleteForward],     mods),
            KC::Tab       => buffer.add_simple(&[Keyboard::Tab],               mods),
            KC::STab      => buffer.add(&[Keyboard::LeftShift, Keyboard::Tab], mods, &[Keyboard::LeftShift], 0),
            KC::Home      => buffer.add_simple(&[Keyboard::Home],              mods),
            KC::End       => buffer.add_simple(&[Keyboard::End],               mods),
            KC::PageUp    => buffer.add_simple(&[Keyboard::PageUp],            mods),
            KC::PageDown  => buffer.add_simple(&[Keyboard::PageDown],          mods),

            KC::Left      => buffer.add_simple(&[Keyboard::LeftArrow],         mods),
            KC::Down      => buffer.add_simple(&[Keyboard::DownArrow],         mods),
            KC::Up        => buffer.add_simple(&[Keyboard::UpArrow],           mods),
            KC::Right     => buffer.add_simple(&[Keyboard::RightArrow],        mods),

            KC::Num0 => buffer.add(&[Keyboard::Keyboard0], mods, &[Keyboard::LeftShift], 0),
            KC::Num1 => buffer.add(&[Keyboard::Keyboard1], mods, &[Keyboard::LeftShift], 0),
            KC::Num2 => buffer.add(&[Keyboard::Keyboard2], mods, &[Keyboard::LeftShift], 0),
            KC::Num3 => buffer.add(&[Keyboard::Keyboard3], mods, &[Keyboard::LeftShift], 0),
            KC::Num4 => buffer.add(&[Keyboard::Keyboard4], mods, &[Keyboard::LeftShift], 0),
            KC::Num5 => buffer.add(&[Keyboard::Keyboard5], mods, &[Keyboard::LeftShift], 0),
            KC::Num6 => buffer.add(&[Keyboard::Keyboard6], mods, &[Keyboard::LeftShift], 0),
            KC::Num7 => buffer.add(&[Keyboard::Keyboard7], mods, &[Keyboard::LeftShift], 0),
            KC::Num8 => buffer.add(&[Keyboard::Keyboard8], mods, &[Keyboard::LeftShift], 0),
            KC::Num9 => buffer.add(&[Keyboard::Keyboard9], mods, &[Keyboard::LeftShift], 0),

            KC::Minus          => buffer.add(&[Keyboard::Minus],          mods, &[Keyboard::LeftShift], 0),
            KC::Equal          => buffer.add(&[Keyboard::Equal],          mods, &[Keyboard::LeftShift], 0),
            KC::LeftBracket    => buffer.add(&[Keyboard::LeftBrace],      mods, &[Keyboard::LeftShift], 0),
            KC::RightBracket   => buffer.add(&[Keyboard::RightBrace],     mods, &[Keyboard::LeftShift], 0),
            KC::Backslash      => buffer.add(&[Keyboard::Backslash],      mods, &[Keyboard::LeftShift], 0),
            KC::NonusHash      => buffer.add(&[Keyboard::NonUSHash],      mods, &[Keyboard::LeftShift], 0),
            KC::SemiColon      => buffer.add(&[Keyboard::Semicolon],      mods, &[Keyboard::LeftShift], 0),
            KC::Quote          => buffer.add(&[Keyboard::Apostrophe],     mods, &[Keyboard::LeftShift], 0),
            KC::Grave          => buffer.add(&[Keyboard::Grave],          mods, &[Keyboard::LeftShift], 0),
            KC::Comma          => buffer.add(&[Keyboard::Comma],          mods, &[Keyboard::LeftShift], 0),
            KC::Dot            => buffer.add(&[Keyboard::Dot],            mods, &[Keyboard::LeftShift], 0),
            KC::Slash          => buffer.add(&[Keyboard::ForwardSlash],   mods, &[Keyboard::LeftShift], 0),
            KC::NonusBackslash => buffer.add(&[Keyboard::NonUSBackslash], mods, &[Keyboard::LeftShift], 0),

            KC::Tilde       => buffer.add(&[Keyboard::LeftShift, Keyboard::Grave],        mods, &[Keyboard::LeftShift], 0),
            KC::Exclaim     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard1],    mods, &[Keyboard::LeftShift], 0),
            KC::At          => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard2],    mods, &[Keyboard::LeftShift], 0),
            KC::Hash        => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard3],    mods, &[Keyboard::LeftShift], 0),
            KC::Dollar      => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard4],    mods, &[Keyboard::LeftShift], 0),
            KC::Percentage  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard5],    mods, &[Keyboard::LeftShift], 0),
            KC::Circumflex  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard6],    mods, &[Keyboard::LeftShift], 0),
            KC::Ampersand   => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard7],    mods, &[Keyboard::LeftShift], 0),
            KC::Asterix     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard8],    mods, &[Keyboard::LeftShift], 0),
            KC::LeftParent  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard9],    mods, &[Keyboard::LeftShift], 0),
            KC::RightParent => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard0],    mods, &[Keyboard::LeftShift], 0),
            KC::Underscore  => buffer.add(&[Keyboard::LeftShift, Keyboard::Minus],        mods, &[Keyboard::LeftShift], 0),
            KC::Plus        => buffer.add(&[Keyboard::LeftShift, Keyboard::Equal],        mods, &[Keyboard::LeftShift], 0),
            KC::LeftCurly   => buffer.add(&[Keyboard::LeftShift, Keyboard::LeftBrace],    mods, &[Keyboard::LeftShift], 0),
            KC::RightCurly  => buffer.add(&[Keyboard::LeftShift, Keyboard::RightBrace],   mods, &[Keyboard::LeftShift], 0),
            KC::Pipe        => buffer.add(&[Keyboard::LeftShift, Keyboard::Backslash],    mods, &[Keyboard::LeftShift], 0),
            KC::Colon       => buffer.add(&[Keyboard::LeftShift, Keyboard::Semicolon],    mods, &[Keyboard::LeftShift], 0),
            KC::DoubleQuote => buffer.add(&[Keyboard::LeftShift, Keyboard::Apostrophe],   mods, &[Keyboard::LeftShift], 0),
            KC::LowerThan   => buffer.add(&[Keyboard::LeftShift, Keyboard::Comma],        mods, &[Keyboard::LeftShift], 0),
            KC::GreaterThan => buffer.add(&[Keyboard::LeftShift, Keyboard::Dot],          mods, &[Keyboard::LeftShift], 0),
            KC::Question    => buffer.add(&[Keyboard::LeftShift, Keyboard::ForwardSlash], mods, &[Keyboard::LeftShift], 0),

            KC::GuillemetL   => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftBrace],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::GuillemetD   => buffer.add(&[Keyboard::RightAlt, Keyboard::RightBrace],                     mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Diameter     => buffer.add(&[Keyboard::RightAlt, Keyboard::L],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Degre        => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Semicolon], mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Euro         => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard5],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Pound        => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard4], mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Copyright    => buffer.add(&[Keyboard::RightAlt, Keyboard::C],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Registered   => buffer.add(&[Keyboard::RightAlt, Keyboard::V],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::ExpOne       => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard1],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::ExpTwo       => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard2],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::ExpThree     => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard3],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Pilcrow      => buffer.add(&[Keyboard::RightAlt, Keyboard::Semicolon],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Multi        => buffer.add(&[Keyboard::RightAlt, Keyboard::Equal],                          mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Div          => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Equal],     mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Beta         => buffer.add(&[Keyboard::RightAlt, Keyboard::S],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Quarter      => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard6], mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Half         => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard7], mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::ThreeQuarter => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard8], mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),
            KC::Yen          => buffer.add(&[Keyboard::RightAlt, Keyboard::Minus],                          mods, &[Keyboard::RightAlt, Keyboard::LeftShift], 0),

            // --
            KC::F1  => buffer.add_simple(&[Keyboard::F1],  mods),
            KC::F2  => buffer.add_simple(&[Keyboard::F2],  mods),
            KC::F3  => buffer.add_simple(&[Keyboard::F3],  mods),
            KC::F4  => buffer.add_simple(&[Keyboard::F4],  mods),
            KC::F5  => buffer.add_simple(&[Keyboard::F5],  mods),
            KC::F6  => buffer.add_simple(&[Keyboard::F6],  mods),
            KC::F7  => buffer.add_simple(&[Keyboard::F7],  mods),
            KC::F8  => buffer.add_simple(&[Keyboard::F8],  mods),
            KC::F9  => buffer.add_simple(&[Keyboard::F9],  mods),
            KC::F10 => buffer.add_simple(&[Keyboard::F10], mods),
            KC::F11 => buffer.add_simple(&[Keyboard::F11], mods),
            KC::F12 => buffer.add_simple(&[Keyboard::F12], mods),
            KC::F13 => buffer.add_simple(&[Keyboard::F13], mods),
            KC::F14 => buffer.add_simple(&[Keyboard::F14], mods),
            KC::F15 => buffer.add_simple(&[Keyboard::F15], mods),
            KC::F16 => buffer.add_simple(&[Keyboard::F16], mods),
            KC::F17 => buffer.add_simple(&[Keyboard::F17], mods),
            KC::F18 => buffer.add_simple(&[Keyboard::F18], mods),
            KC::F19 => buffer.add_simple(&[Keyboard::F19], mods),
            KC::F20 => buffer.add_simple(&[Keyboard::F20], mods),
            KC::F21 => buffer.add_simple(&[Keyboard::F21], mods),
            KC::F22 => buffer.add_simple(&[Keyboard::F22], mods),
            KC::F23 => buffer.add_simple(&[Keyboard::F23], mods),
            KC::F24 => buffer.add_simple(&[Keyboard::F24], mods),

            // --
            KC::ACircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::ADiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::AGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::ECircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::EDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::EGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::ICircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::IDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::IGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::OCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::ODiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::OGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::UCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::UDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::UGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::YCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX, TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::YDiaer  => buffer.add_no_mods(&DEAD_DIAERIS,    TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::YGrave  => buffer.add_no_mods(&DEAD_GRAVE,      TEMPO_DEAD_KEY).add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated], 0),

            KC::Qu   => buffer.add(&[Keyboard::Q, Keyboard::U], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl], 0).add_no_mods(&[Keyboard::NoEventIndicated], 0),
            KC::Tion => buffer.add(&[Keyboard::T], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl], 0)
                              .add(&[Keyboard::I], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl], 0)
                              .add(&[Keyboard::O], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl], 0)
                              .add(&[Keyboard::N], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl], 0)
                              .add_no_mods(&[Keyboard::NoEventIndicated], 0),

            // --
            KC::Alt   => buffer.add_no_mods(&[Keyboard::LeftAlt],     0),
            KC::Altgr => buffer.add_no_mods(&[Keyboard::RightAlt],    0),
            KC::Ctl   => buffer.add_no_mods(&[Keyboard::LeftControl], 0),
            KC::Gui   => buffer.add_no_mods(&[Keyboard::LeftGUI],     0),
            KC::Sft   => buffer.add_no_mods(&[Keyboard::LeftShift],   0),

            KC::HomeRow(_, k) => k.usb_code(buffer, mods),

            // --
            // Use ↵ to enter
            KC::MacroGit       => str_to_usb_code("flinguenheld",                              buffer),
            KC::MacroMailShort => str_to_usb_code("f@linguenheld.fr",                          buffer),
            KC::MacroMail      => str_to_usb_code("florent@linguenheld.fr",                    buffer),
            KC::MacroHTTPS     => str_to_usb_code("https://",                                  buffer),
            KC::MacroDotfiles  => str_to_usb_code("https://github.com/flinguenheld/dotfiles",  buffer),
            KC::MacroNixOS     => str_to_usb_code("sudo nixos-rebuild switch --flake .#flopc", buffer),

            _ => buffer,
        }
    }
}

#![cfg_attr(rustfmt, rustfmt_skip)]
use heapless::{Deque, Vec};
use usbd_human_interface_device::page::Keyboard;

use crate::options::{BUFFER_CASE_LENGTH, BUFFER_LENGTH};
use super::{macros::str_to_usb_code, modifiers::Modifiers};

// --------------------------------------------------------------------------------------
const DEAD_CIRCUMFLEX: [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Keyboard6];
const DEAD_DIAERIS:    [Keyboard; 3] = [Keyboard::LeftShift, Keyboard::RightAlt, Keyboard::Apostrophe, ];
const DEAD_GRAVE:      [Keyboard; 2] = [Keyboard::RightAlt,  Keyboard::Grave];

// --------------------------------------------------------------------------------------
// Help KC conversion along.
// This buffer is filled here to be then emptied by the writing report.
// Each entry is a full report: a vec of Keyboard pages.
#[derive(PartialEq, Default)]
pub struct BuffCase {
    pub key_code: Vec<Keyboard, BUFFER_CASE_LENGTH>,
}

pub struct Buffer {
    pub keys: Deque<BuffCase, BUFFER_LENGTH>,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer { keys: Deque::new() }
    }

    // Add a new entry in the Deque with active mods (without excluded) and the keys.
    // The same case as the last one queued gets a release before it, else the second would
    // be dropped as a duplicate (see UsbReports::write_keyboard): "!!", "ss" in a macro...
    fn add(mut self, keys: &[Keyboard], mods: &Modifiers, excluded_mods: &[Keyboard]) -> Self {
        let mut key_code = Vec::new();
        key_code.extend(mods.active()
                         .iter()
                         .filter(|m| !excluded_mods.contains(m))
                         .copied());
        key_code.extend(keys.iter().copied());

        let case = BuffCase {key_code};
        if self.keys.back() == Some(&case) && case.key_code != [Keyboard::NoEventIndicated] {
            self = self.add_no_mods(&[Keyboard::NoEventIndicated]);
        }

        self.keys.push_back(case).ok();
        self
    }

    fn add_simple(self, keys: &[Keyboard], mods: &Modifiers) -> Self {
        self.add(keys, mods, &[])
    }

    fn add_no_mods(self, keys: &[Keyboard]) -> Self {
        self.add(keys, &Modifiers::new(), &[])
    }

    // Add the report state built by chew (mods + held keys), as it is.
    // No release before a same state: it means the keys are still held.
    pub fn add_state(mut self, codes: &[Keyboard]) -> Self {
        let mut key_code = Vec::new();
        key_code.extend(codes.iter().copied());

        self.keys.push_back(BuffCase {key_code}).ok();
        self
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
         
        Yen = 3039,

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
    MacroMailEnd = 52003,
    MacroMailShort = 52004,
    MacroHTTPS = 52005,
    MacroDotfiles = 52006,
    MacroNixOS = 52007,

        ToggleBuzzer = 60000,
}

impl KC {
    /// Convert a Chew keycode into an array of Keyboard page.
    pub fn usb_code(&self, buffer: Buffer, mods: &Modifiers) -> Buffer {
        if self.is_nkro_key() {
            return buffer.add_simple(&[self.nkro_code()], mods);
        }

        match *self {
            KC::None => buffer.add_simple(&[Keyboard::NoEventIndicated], mods),

            KC::CCedilla => buffer.add(&[Keyboard::RightAlt, Keyboard::Comma], mods, &[Keyboard::RightAlt]),
            KC::EAcute   => buffer.add(&[Keyboard::RightAlt, Keyboard::E],     mods, &[Keyboard::RightAlt]),
            KC::AE       => buffer.add(&[Keyboard::RightAlt, Keyboard::Z],     mods, &[Keyboard::RightAlt]),
            KC::OE       => buffer.add(&[Keyboard::RightAlt, Keyboard::K],     mods, &[Keyboard::RightAlt]),

            KC::STab      => buffer.add(&[Keyboard::LeftShift, Keyboard::Tab], mods, &[Keyboard::LeftShift]),

            KC::Num0 => buffer.add(&[Keyboard::Keyboard0], mods, &[Keyboard::LeftShift]),
            KC::Num1 => buffer.add(&[Keyboard::Keyboard1], mods, &[Keyboard::LeftShift]),
            KC::Num2 => buffer.add(&[Keyboard::Keyboard2], mods, &[Keyboard::LeftShift]),
            KC::Num3 => buffer.add(&[Keyboard::Keyboard3], mods, &[Keyboard::LeftShift]),
            KC::Num4 => buffer.add(&[Keyboard::Keyboard4], mods, &[Keyboard::LeftShift]),
            KC::Num5 => buffer.add(&[Keyboard::Keyboard5], mods, &[Keyboard::LeftShift]),
            KC::Num6 => buffer.add(&[Keyboard::Keyboard6], mods, &[Keyboard::LeftShift]),
            KC::Num7 => buffer.add(&[Keyboard::Keyboard7], mods, &[Keyboard::LeftShift]),
            KC::Num8 => buffer.add(&[Keyboard::Keyboard8], mods, &[Keyboard::LeftShift]),
            KC::Num9 => buffer.add(&[Keyboard::Keyboard9], mods, &[Keyboard::LeftShift]),

            KC::Minus          => buffer.add(&[Keyboard::Minus],          mods, &[Keyboard::LeftShift]),
            KC::Equal          => buffer.add(&[Keyboard::Equal],          mods, &[Keyboard::LeftShift]),
            KC::LeftBracket    => buffer.add(&[Keyboard::LeftBrace],      mods, &[Keyboard::LeftShift]),
            KC::RightBracket   => buffer.add(&[Keyboard::RightBrace],     mods, &[Keyboard::LeftShift]),
            KC::Backslash      => buffer.add(&[Keyboard::Backslash],      mods, &[Keyboard::LeftShift]),
            KC::NonusHash      => buffer.add(&[Keyboard::NonUSHash],      mods, &[Keyboard::LeftShift]),
            KC::SemiColon      => buffer.add(&[Keyboard::Semicolon],      mods, &[Keyboard::LeftShift]),
            KC::Quote          => buffer.add(&[Keyboard::Apostrophe],     mods, &[Keyboard::LeftShift]),
            KC::Grave          => buffer.add(&[Keyboard::Grave],          mods, &[Keyboard::LeftShift]),
            KC::Comma          => buffer.add(&[Keyboard::Comma],          mods, &[Keyboard::LeftShift]),
            KC::Dot            => buffer.add(&[Keyboard::Dot],            mods, &[Keyboard::LeftShift]),
            KC::Slash          => buffer.add(&[Keyboard::ForwardSlash],   mods, &[Keyboard::LeftShift]),
            KC::NonusBackslash => buffer.add(&[Keyboard::NonUSBackslash], mods, &[Keyboard::LeftShift]),

            KC::Tilde       => buffer.add(&[Keyboard::LeftShift, Keyboard::Grave],        mods, &[Keyboard::LeftShift]),
            KC::Exclaim     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard1],    mods, &[Keyboard::LeftShift]),
            KC::At          => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard2],    mods, &[Keyboard::LeftShift]),
            KC::Hash        => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard3],    mods, &[Keyboard::LeftShift]),
            KC::Dollar      => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard4],    mods, &[Keyboard::LeftShift]),
            KC::Percentage  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard5],    mods, &[Keyboard::LeftShift]),
            KC::Circumflex  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard6],    mods, &[Keyboard::LeftShift]),
            KC::Ampersand   => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard7],    mods, &[Keyboard::LeftShift]),
            KC::Asterix     => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard8],    mods, &[Keyboard::LeftShift]),
            KC::LeftParent  => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard9],    mods, &[Keyboard::LeftShift]),
            KC::RightParent => buffer.add(&[Keyboard::LeftShift, Keyboard::Keyboard0],    mods, &[Keyboard::LeftShift]),
            KC::Underscore  => buffer.add(&[Keyboard::LeftShift, Keyboard::Minus],        mods, &[Keyboard::LeftShift]),
            KC::Plus        => buffer.add(&[Keyboard::LeftShift, Keyboard::Equal],        mods, &[Keyboard::LeftShift]),
            KC::LeftCurly   => buffer.add(&[Keyboard::LeftShift, Keyboard::LeftBrace],    mods, &[Keyboard::LeftShift]),
            KC::RightCurly  => buffer.add(&[Keyboard::LeftShift, Keyboard::RightBrace],   mods, &[Keyboard::LeftShift]),
            KC::Pipe        => buffer.add(&[Keyboard::LeftShift, Keyboard::Backslash],    mods, &[Keyboard::LeftShift]),
            KC::Colon       => buffer.add(&[Keyboard::LeftShift, Keyboard::Semicolon],    mods, &[Keyboard::LeftShift]),
            KC::DoubleQuote => buffer.add(&[Keyboard::LeftShift, Keyboard::Apostrophe],   mods, &[Keyboard::LeftShift]),
            KC::LowerThan   => buffer.add(&[Keyboard::LeftShift, Keyboard::Comma],        mods, &[Keyboard::LeftShift]),
            KC::GreaterThan => buffer.add(&[Keyboard::LeftShift, Keyboard::Dot],          mods, &[Keyboard::LeftShift]),
            KC::Question    => buffer.add(&[Keyboard::LeftShift, Keyboard::ForwardSlash], mods, &[Keyboard::LeftShift]),

            KC::GuillemetL   => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftBrace],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::GuillemetD   => buffer.add(&[Keyboard::RightAlt, Keyboard::RightBrace],                     mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Diameter     => buffer.add(&[Keyboard::RightAlt, Keyboard::L],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Degre        => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Semicolon], mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Euro         => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard5],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Pound        => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard4], mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Copyright    => buffer.add(&[Keyboard::RightAlt, Keyboard::C],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Registered   => buffer.add(&[Keyboard::RightAlt, Keyboard::V],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::ExpOne       => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard1],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::ExpTwo       => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard2],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::ExpThree     => buffer.add(&[Keyboard::RightAlt, Keyboard::Keyboard3],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Pilcrow      => buffer.add(&[Keyboard::RightAlt, Keyboard::Semicolon],                      mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Multi        => buffer.add(&[Keyboard::RightAlt, Keyboard::Equal],                          mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Div          => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Equal],     mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Beta         => buffer.add(&[Keyboard::RightAlt, Keyboard::S],                              mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Quarter      => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard6], mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Half         => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard7], mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::ThreeQuarter => buffer.add(&[Keyboard::RightAlt, Keyboard::LeftShift, Keyboard::Keyboard8], mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),
            KC::Yen          => buffer.add(&[Keyboard::RightAlt, Keyboard::Minus],                          mods, &[Keyboard::RightAlt, Keyboard::LeftShift]),

            // --
            KC::ACircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::ADiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::AGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::A], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::ECircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::EDiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::EGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::E], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::ICircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::IDiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::IGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::I], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::OCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::ODiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::OGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::O], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::UCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::UDiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::UGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::U], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::YCircum => buffer.add_no_mods(&DEAD_CIRCUMFLEX).add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::YDiaer  => buffer.add_no_mods(&DEAD_DIAERIS)   .add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::YGrave  => buffer.add_no_mods(&DEAD_GRAVE)     .add_simple(&[Keyboard::Y], mods).add_no_mods(&[Keyboard::NoEventIndicated]),

            KC::Qu   => buffer.add(&[Keyboard::Q, Keyboard::U], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl]).add_no_mods(&[Keyboard::NoEventIndicated]),
            KC::Tion => buffer.add(&[Keyboard::T], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl])
                              .add(&[Keyboard::I], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl])
                              .add(&[Keyboard::O], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl])
                              .add(&[Keyboard::N], mods, &[Keyboard::LeftAlt, Keyboard::RightAlt, Keyboard::LeftGUI, Keyboard::LeftControl])
                              .add_no_mods(&[Keyboard::NoEventIndicated]),

            // --
            KC::Alt   => buffer.add_no_mods(&[Keyboard::LeftAlt]),
            KC::Altgr => buffer.add_no_mods(&[Keyboard::RightAlt]),
            KC::Ctl   => buffer.add_no_mods(&[Keyboard::LeftControl]),
            KC::Gui   => buffer.add_no_mods(&[Keyboard::LeftGUI]),
            KC::Sft   => buffer.add_no_mods(&[Keyboard::LeftShift]),

            KC::HomeRow(_, k) => k.usb_code(buffer, mods),

            // --
            // Use ↵ to enter
            KC::MacroGit       => str_to_usb_code("flinguenheld",                              buffer),
            KC::MacroMailShort => str_to_usb_code("f@linguenheld.net",                         buffer),
            KC::MacroMail      => str_to_usb_code("florent@linguenheld.net",                   buffer),
            KC::MacroMailEnd   => str_to_usb_code("@linguenheld.net",                          buffer),
            KC::MacroHTTPS     => str_to_usb_code("https://",                                  buffer),
            KC::MacroDotfiles  => str_to_usb_code("https://github.com/flinguenheld/dotfiles",  buffer),
            KC::MacroNixOS     => str_to_usb_code("sudo nixos-rebuild switch --flake .#flopc", buffer),

            _ => buffer,
        }
    }
}

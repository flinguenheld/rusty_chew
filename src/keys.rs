use crate::utils::{modifiers::Modifiers, options::BUFFER_LENGTH};
use heapless::Deque;
use usbd_human_interface_device::{device::mouse::WheelMouseReport, page::Keyboard};

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

#[derive(Clone)]
pub enum Lay {
    Pressed(usize, usize),
    Dead(usize, usize, bool),
}

#[rustfmt::skip]
#[allow(dead_code)]
#[repr(u16)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

    // Mouse
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
}

#[rustfmt::skip]
impl KC {

    // Mouse ----------------------------------------------------------------------------
    pub fn usb_mouse_move(&self, mut report: WheelMouseReport, speed: i8) -> WheelMouseReport 
    {
        match *self {
            KC::MouseLeft  => report.x = i8::saturating_add(report.x, -speed),
            KC::MouseDown  => report.y = i8::saturating_add(report.y,  speed),
            KC::MouseUp    => report.y = i8::saturating_add(report.y, -speed),
            KC::MouseRight => report.x = i8::saturating_add(report.x,  speed),

            KC::MouseWheelLeft  => report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel,  speed),
            KC::MouseWheelDown  => report.vertical_wheel   = i8::saturating_add(report.vertical_wheel,   -speed),
            KC::MouseWheelUp    => report.vertical_wheel   = i8::saturating_add(report.vertical_wheel,    speed),
            KC::MouseWheelRight => report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel, -speed),
            _=>{}
        }
        report
    }

    // Keyboard -------------------------------------------------------------------------
    fn new_combination(&self, modifiers: &Modifiers) -> [Keyboard; 6] {
        let mut output = EMPTY;

        // Exclude numbers and symbols from shift
        if (modifiers.shift.0 || *self == KC::Shift) && (*self < KC::Num0 || *self > KC::Yen) { output[0] = Keyboard::LeftShift; }
        if modifiers.alt.0    || *self == KC::Alt {                                             output[1] = Keyboard::LeftAlt; }
        if modifiers.alt_gr.0 || *self == KC::Altgr {                                           output[2] = Keyboard::RightAlt; }
        if modifiers.ctrl.0   || *self == KC::Ctrl {                                            output[3] = Keyboard::LeftControl; }
        if modifiers.gui.0    || *self == KC::Gui {                                             output[4] = Keyboard::LeftGUI; }

        output
    }

    /// Convert a Chew keycode into an array of Keyboard page.
    /// It allows the usb write report to print each key with its modifiers.
    #[rustfmt::skip]
    pub fn usb_code(&self, modifiers: &Modifiers, buffer: &mut Deque<[Keyboard; 6], BUFFER_LENGTH>) {

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

            KC::CCedilla   => { output[2] = Keyboard::RightAlt; output[5] = Keyboard::Comma;      buffer.push_back(output).ok(); },
            KC::EAcute     => { output[2] = Keyboard::RightAlt; output[5] = Keyboard::E;          buffer.push_back(output).ok(); },
            KC::AE         => { output[2] = Keyboard::RightAlt; output[5] = Keyboard::Z;          buffer.push_back(output).ok(); },
            KC::OE         => { output[2] = Keyboard::RightAlt; output[5] = Keyboard::K;          buffer.push_back(output).ok(); },

            KC::Enter     => {                                  output[5] = Keyboard::ReturnEnter;     buffer.push_back(output).ok(); },
            KC::Space     => {                                  output[5] = Keyboard::Space;           buffer.push_back(output).ok(); },
            KC::Esc       => {                                  output[5] = Keyboard::Escape;          buffer.push_back(output).ok(); },
            KC::Del       => {                                  output[5] = Keyboard::DeleteBackspace; buffer.push_back(output).ok(); },
            KC::BackSpace => {                                  output[5] = Keyboard::DeleteForward;   buffer.push_back(output).ok(); },
            KC::Tab       => {                                  output[5] = Keyboard::Tab;             buffer.push_back(output).ok(); },
            KC::STab      => { output[0] = Keyboard::LeftShift; output[5] = Keyboard::Tab;             buffer.push_back(output).ok(); },
            KC::Home      => {                                  output[5] = Keyboard::Home;            buffer.push_back(output).ok(); },
            KC::End       => {                                  output[5] = Keyboard::End;             buffer.push_back(output).ok(); },
            KC::PageUp    => {                                  output[5] = Keyboard::PageUp;          buffer.push_back(output).ok(); },
            KC::PageDown  => {                                  output[5] = Keyboard::PageDown;        buffer.push_back(output).ok(); },

            KC::Left  => { output[5] = Keyboard::LeftArrow;  buffer.push_back(output).ok(); },
            KC::Down  => { output[5] = Keyboard::DownArrow;  buffer.push_back(output).ok(); },
            KC::Up    => { output[5] = Keyboard::UpArrow;    buffer.push_back(output).ok(); },
            KC::Right => { output[5] = Keyboard::RightArrow; buffer.push_back(output).ok(); },

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

            KC::GuillemetL => { output[2] = Keyboard::RightAlt;  output[5] = Keyboard::LeftBrace;                                   buffer.push_back(output).ok(); },
            KC::GuillemetD => { output[2] = Keyboard::RightAlt;  output[5] = Keyboard::RightBrace;                                  buffer.push_back(output).ok(); },
            KC::Diameter   => { output[2] = Keyboard::RightAlt;  output[5] = Keyboard::L;                                           buffer.push_back(output).ok(); },
            KC::Degre      => { output[0] = Keyboard::LeftShift; output[2] = Keyboard::RightAlt;   output[5] = Keyboard::Semicolon; buffer.push_back(output).ok(); },
            KC::Euro       => { output[2] = Keyboard::RightAlt;  output[5] = Keyboard::Keyboard5;                                   buffer.push_back(output).ok(); },
            KC::Pound      => { output[0] = Keyboard::LeftShift; output[2] = Keyboard::RightAlt;   output[5] = Keyboard::Keyboard4; buffer.push_back(output).ok(); },
            KC::Yen        => { output[2] = Keyboard::RightAlt;  output[5] = Keyboard::Minus;                                       buffer.push_back(output).ok(); },

            KC::HomeAltA  => { output[5] = Keyboard::A; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeAltU  => { output[5] = Keyboard::U; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeGuiS  => { output[5] = Keyboard::S; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeGuiI  => { output[5] = Keyboard::I; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeCtrlE => { output[5] = Keyboard::E; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeCtrlT => { output[5] = Keyboard::T; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeSftN  => { output[5] = Keyboard::N; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }
            KC::HomeSftR  => { output[5] = Keyboard::R; buffer.push_back(output).ok(); buffer.push_back(EMPTY).ok(); }

            _ => {}
        }
    }
}

use heapless::Vec;
use usbd_human_interface_device::page::Keyboard;

use super::{
    keys::{Buffer, KC},
    modifiers::Modifiers,
};
use crate::{
    hardware::matrix::Matrix,
    options::{BUFFER_CASE_LENGTH, NKRO_MAX_KEYS},
};

/// NKRO: several keys held at once, without losing the hacks.
/// The report (mods + held keys) is pushed only when it changes:
///
///     press   b  ->  [B]
///     press   a  ->  [B, A]     (one key at a time for now, the new one replaces the others)
///     release b  ->  [A]
///     release a  ->  [NoEvent]
///
/// Keys with their own modifiers (symbols, é...) can't share a report, see docs/NKRO_PLAN.md.
pub struct Nkro {
    // NKRO keys typed and still down: matrix index + USB code
    held: Vec<(usize, Keyboard), NKRO_MAX_KEYS>,
    // One-case hack typed and still down (symbol, number, é...), alone in its report
    // until its release
    solo_key: Option<usize>,
    // Last report pushed
    last_report: Vec<Keyboard, BUFFER_CASE_LENGTH>,
}

impl Nkro {
    pub fn new() -> Self {
        Nkro {
            held: Vec::new(),
            solo_key: None,
            // The computer starts with nothing pressed
            last_report: Vec::from_slice(&[Keyboard::NoEventIndicated]).unwrap(),
        }
    }

    /// Called right after a key is typed: an NKRO key still down is kept, a one-case hack
    /// still down keeps its report, anything else (released, sequence, leader result) is a tap.
    pub fn hold(&mut self, matrix: &Matrix, code: KC, index: usize) {
        let code = match code {
            KC::HomeRow(_, k) => *k,
            c => c,
        };

        // One key at a time, the new one replaces the others
        self.held.clear();
        self.solo_key = None;

        if matrix.is_active(index) {
            if code.is_nkro_key() {
                self.held.push((index, code.nkro_code())).ok();
            } else if code < KC::ACircum {
                self.solo_key = Some(index);
            }
        }
    }

    /// End of the chew run: released keys leave, the report is pushed if it changed.
    /// buffer_len is the buffer length at the start of the run.
    pub fn update(
        &mut self,
        mut key_buffer: Buffer,
        buffer_len: usize,
        matrix: &Matrix,
        mods: &Modifiers,
    ) -> Buffer {
        // A case pushed by this run is what the computer has now
        if key_buffer.keys.len() > buffer_len {
            if let Some(case) = key_buffer.keys.back() {
                self.last_report = case.key_code.clone();
            }
        }

        // Released keys leave the report
        self.held.retain(|(index, _)| matrix.is_active(*index));
        if self.solo_key.is_some_and(|index| !matrix.is_active(index)) {
            self.solo_key = None;
        }

        // Mods + held keys, pushed when they changed (a solo key keeps its own case).
        // The mods stay without key, useful for the real mouse.
        if self.solo_key.is_none() {
            let report = self.report(mods);
            if report != self.last_report {
                key_buffer = key_buffer.add_state(&report);
                self.last_report = report;
            }
        }

        key_buffer
    }

    /// Mods + held keys, NoEventIndicated when nothing is pressed.
    fn report(&self, mods: &Modifiers) -> Vec<Keyboard, BUFFER_CASE_LENGTH> {
        let mut report = Vec::new();
        for code in mods.active().iter().chain(self.held.iter().map(|(_, c)| c)) {
            report.push(*code).ok();
        }
        if report.is_empty() {
            report.push(Keyboard::NoEventIndicated).ok();
        }
        report
    }
}

impl KC {
    /// Keys with one code and no modifiers of their own, they can be held together (NKRO).
    /// The enum order is used by other filters (leader, typed, held), so the gaps are handled here.
    pub fn is_nkro_key(&self) -> bool {
        (*self >= KC::A && *self <= KC::Z)
            || (*self >= KC::Enter && *self <= KC::Right && *self != KC::STab)
            || (*self >= KC::F1 && *self <= KC::F24)
    }

    /// USB code of an NKRO key (see is_nkro_key), NoEventIndicated for the others.
    #[rustfmt::skip]
    pub fn nkro_code(&self) -> Keyboard {
        match *self {
            KC::A => Keyboard::A,
            KC::B => Keyboard::B,
            KC::C => Keyboard::C,
            KC::D => Keyboard::D,
            KC::E => Keyboard::E,
            KC::F => Keyboard::F,
            KC::G => Keyboard::G,
            KC::H => Keyboard::H,
            KC::I => Keyboard::I,
            KC::J => Keyboard::J,
            KC::K => Keyboard::K,
            KC::L => Keyboard::L,
            KC::M => Keyboard::M,
            KC::N => Keyboard::N,
            KC::O => Keyboard::O,
            KC::P => Keyboard::P,
            KC::Q => Keyboard::Q,
            KC::R => Keyboard::R,
            KC::S => Keyboard::S,
            KC::T => Keyboard::T,
            KC::U => Keyboard::U,
            KC::V => Keyboard::V,
            KC::W => Keyboard::W,
            KC::X => Keyboard::X,
            KC::Y => Keyboard::Y,
            KC::Z => Keyboard::Z,

            KC::Enter     => Keyboard::ReturnEnter,
            KC::Space     => Keyboard::Space,
            KC::Esc       => Keyboard::Escape,
            KC::Del       => Keyboard::DeleteBackspace,
            KC::BackSpace => Keyboard::DeleteForward,
            KC::Tab       => Keyboard::Tab,
            KC::Home      => Keyboard::Home,
            KC::End       => Keyboard::End,
            KC::PageUp    => Keyboard::PageUp,
            KC::PageDown  => Keyboard::PageDown,

            KC::Left      => Keyboard::LeftArrow,
            KC::Down      => Keyboard::DownArrow,
            KC::Up        => Keyboard::UpArrow,
            KC::Right     => Keyboard::RightArrow,

            KC::F1  => Keyboard::F1,
            KC::F2  => Keyboard::F2,
            KC::F3  => Keyboard::F3,
            KC::F4  => Keyboard::F4,
            KC::F5  => Keyboard::F5,
            KC::F6  => Keyboard::F6,
            KC::F7  => Keyboard::F7,
            KC::F8  => Keyboard::F8,
            KC::F9  => Keyboard::F9,
            KC::F10 => Keyboard::F10,
            KC::F11 => Keyboard::F11,
            KC::F12 => Keyboard::F12,
            KC::F13 => Keyboard::F13,
            KC::F14 => Keyboard::F14,
            KC::F15 => Keyboard::F15,
            KC::F16 => Keyboard::F16,
            KC::F17 => Keyboard::F17,
            KC::F18 => Keyboard::F18,
            KC::F19 => Keyboard::F19,
            KC::F20 => Keyboard::F20,
            KC::F21 => Keyboard::F21,
            KC::F22 => Keyboard::F22,
            KC::F23 => Keyboard::F23,
            KC::F24 => Keyboard::F24,

            _ => Keyboard::NoEventIndicated,
        }
    }
}

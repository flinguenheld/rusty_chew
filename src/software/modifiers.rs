#![cfg_attr(rustfmt, rustfmt_skip)]
use heapless::Vec;
use usbd_human_interface_device::page::Keyboard;

use crate::options::NB_KEYS;

use super::{chew::Key, keys::KC};

/// Due to layers modifiers have to be manage with their matrix index directly.
/// This struct keeps the matrix index for each modifier.
pub struct Modifiers {
    pub alt:     usize,
    pub alt_gr:  usize,
    pub ctrl:    usize,
    pub gui:     usize,
    pub shift:   usize,
    pub caplock: bool,
}

impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            alt:    usize::MAX,
            alt_gr: usize::MAX,
            ctrl:   usize::MAX,
            gui:    usize::MAX,
            shift:  usize::MAX,
            caplock: false,
        }
    }

    pub fn set(&mut self, key: KC, index: usize){
        match key {
            KC::Alt     => self.alt     = index,
            KC::Altgr   => self.alt_gr  = index,
            KC::Ctl     => self.ctrl    = index,
            KC::Gui     => self.gui     = index,
            KC::Sft     => self.shift   = index,
            KC::CapLock => self.caplock = !self.caplock,
            _=>{}
        }
    }

    pub fn active(&self) -> Vec<Keyboard, 5> {
        let mut output = Vec::new();
        if self.alt    != usize::MAX { output.push(Keyboard::LeftAlt).ok(); }
        if self.alt_gr != usize::MAX { output.push(Keyboard::RightAlt).ok(); }
        if self.ctrl   != usize::MAX { output.push(Keyboard::LeftControl).ok(); }
        if self.gui    != usize::MAX { output.push(Keyboard::LeftGUI).ok(); }
        if self.shift  != usize::MAX { output.push(Keyboard::LeftShift).ok(); }
        if self.caplock              { output.push(Keyboard::LeftShift).ok(); }

        output
    }

    pub fn active_kc(&self) -> Vec<(KC, usize), 5> {
        let mut output = Vec::new();
        if self.alt     != usize::MAX { output.push((KC::Alt,   self.alt)).ok(); }
        if self.alt_gr  != usize::MAX { output.push((KC::Altgr, self.alt_gr)).ok(); }
        if self.ctrl    != usize::MAX { output.push((KC::Ctl,   self.ctrl)).ok(); }
        if self.gui     != usize::MAX { output.push((KC::Gui,   self.gui)).ok(); }
        if self.shift   != usize::MAX { output.push((KC::Sft,   self.shift)).ok(); }
        if self.caplock               { output.push((KC::Sft,   usize::MAX)).ok(); }

        output
    }    

    pub fn update_state(&mut self, pressed_keys: &Vec<Key, NB_KEYS>) {
        self.alt    = pressed_keys.iter().find(|k| k.code == KC::Alt  ).map(|k| k.index).unwrap_or(usize::MAX);
        self.alt_gr = pressed_keys.iter().find(|k| k.code == KC::Altgr).map(|k| k.index).unwrap_or(usize::MAX);
        self.ctrl   = pressed_keys.iter().find(|k| k.code == KC::Ctl  ).map(|k| k.index).unwrap_or(usize::MAX);
        self.gui    = pressed_keys.iter().find(|k| k.code == KC::Gui  ).map(|k| k.index).unwrap_or(usize::MAX);
        self.shift  = pressed_keys.iter().find(|k| k.code == KC::Sft  ).map(|k| k.index).unwrap_or(usize::MAX);
    }
}

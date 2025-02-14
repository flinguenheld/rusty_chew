#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{chew::Key, keys::KC};
use heapless::Vec;
use usbd_human_interface_device::page::Keyboard;


/// Due to layers modifiers have to be manage with their matrix index directly.
/// This struct keeps the matrix index for each modifier.
pub struct Modifiers {
    pub alt: usize,
    pub alt_gr: usize,
    pub ctrl: usize,
    pub gui: usize,
    pub shift: usize,
}

impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            alt:    usize::MAX,
            alt_gr: usize::MAX,
            ctrl:   usize::MAX,
            gui:    usize::MAX,
            shift:  usize::MAX,
        }
    }

    pub fn set(&mut self, key: KC, index: usize){
        match key {
            KC::Alt   => self.alt    = index,
            KC::Altgr => self.alt_gr = index,
            KC::Ctrl  => self.ctrl   = index,
            KC::Gui   => self.gui    = index,
            KC::Shift => self.shift  = index,
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

        output
    }

    pub fn active_kc(&self) -> Vec<(KC, usize), 5> {
        let mut output = Vec::new();
        if self.alt    != usize::MAX { output.push((KC::Alt,   self.alt)).ok(); }
        if self.alt_gr != usize::MAX { output.push((KC::Altgr, self.alt_gr)).ok(); }
        if self.ctrl   != usize::MAX { output.push((KC::Ctrl,  self.ctrl)).ok(); }
        if self.gui    != usize::MAX { output.push((KC::Gui,   self.gui)).ok(); }
        if self.shift  != usize::MAX { output.push((KC::Shift, self.shift)).ok(); }

        output
    }    

    pub fn update_state(&mut self, pressed_keys: &Vec<Key, 34>) {
        self.alt    = pressed_keys.iter().find(|k| k.code == KC::Alt  ).map(|k| k.index).unwrap_or(usize::MAX);
        self.alt_gr = pressed_keys.iter().find(|k| k.code == KC::Altgr).map(|k| k.index).unwrap_or(usize::MAX);
        self.ctrl   = pressed_keys.iter().find(|k| k.code == KC::Ctrl ).map(|k| k.index).unwrap_or(usize::MAX);
        self.gui    = pressed_keys.iter().find(|k| k.code == KC::Gui  ).map(|k| k.index).unwrap_or(usize::MAX);
        self.shift  = pressed_keys.iter().find(|k| k.code == KC::Shift).map(|k| k.index).unwrap_or(usize::MAX);
    }
}

use heapless::{FnvIndexMap, Vec};

use super::{
    chew::Key,
    keys::{Buffer, KC},
    modifiers::Modifiers,
};
use crate::{
    hardware::led::{LED_DYNMAC_GO_WAIT, LED_DYNMAC_REC, LED_DYNMAC_REC_WAIT},
    options::NB_KEYS,
};

#[derive(PartialEq, Eq)]
enum State {
    Inactive,
    RecordWaitKey,
    RecordInProgress,
    GoWaitKey,
}

/// Allows the user to record some printed keys
/// To record:
///    1 - Press KC::DynMacRecord
///    2 - Press a key which will be used to create an entry in the macro dictionary
///    3 - Now all pressed key are recorded with their modifiers
///    4 - Press KC::DynMacRecord to end up the record
///
/// To launch a record, press KC::DynMacGo and the key which was used to record the macro
///
/// Do not work:
///     - Mouse
///     - Nested dynamic macros (useful ?)
///     - Holding keys
pub struct DynMac {
    state: State,
    key_record: KC,
    macros: FnvIndexMap<KC, Vec<(KC, Modifiers), 128>, 32>, // Is it too much ?
}

impl DynMac {
    pub fn new() -> Self {
        DynMac {
            state: State::Inactive,
            key_record: KC::None,
            macros: FnvIndexMap::new(),
        }
    }

    /// Manage the state according to pressed_keys
    pub fn run(&mut self, pressed_keys: &mut Vec<Key, NB_KEYS>, mut key_buffer: Buffer) -> Buffer {
        match self.state {
            State::Inactive => {
                // Start a new record --
                if let Some(key) = pressed_keys.iter_mut().find(|k| k.code == KC::DynMacRecord) {
                    self.state = State::RecordWaitKey;
                    key.code = KC::Done;

                // Go --
                } else if let Some(key) = pressed_keys.iter_mut().find(|k| k.code == KC::DynMacGo) {
                    self.state = State::GoWaitKey;
                    key.code = KC::Done;
                }
            }

            // Record active, now select the key to save it (erase if it already exists)
            State::RecordWaitKey => {
                if let Some(key) = pressed_keys
                    .iter_mut()
                    .find(|k| k.code > KC::DoneButKeep && k.code < KC::MouseBtLeft)
                {
                    self.macros.insert(key.code, Vec::new()).ok();
                    self.key_record = key.code;
                    self.state = State::RecordInProgress;
                    key.code = KC::Done;
                }
            }

            // Stop recording ?
            State::RecordInProgress => {
                if let Some(key) = pressed_keys.iter_mut().find(|k| k.code == KC::DynMacRecord) {
                    self.state = State::Inactive;
                    key.code = KC::Done;
                }
            }

            // Go active, now select the macro
            State::GoWaitKey => {
                if let Some(key) = pressed_keys
                    .iter_mut()
                    .find(|k| k.code > KC::DoneButKeep && k.code < KC::MouseBtLeft)
                {
                    // Then launch it by filling the key buffer --
                    if let Some(list) = self.macros.get(&key.code) {
                        let mut previous = KC::None;
                        for (k, m) in list.iter() {
                            if previous == *k {
                                // Add a break to allow twice same key
                                key_buffer = KC::None.usb_code(key_buffer, &Modifiers::new());
                            }
                            key_buffer = k.usb_code(key_buffer, m);
                            previous = *k;
                        }
                        key_buffer = KC::None.usb_code(key_buffer, &Modifiers::new());
                    }

                    self.state = State::Inactive;
                    key.code = KC::Done;
                }
            }
        }

        key_buffer
    }

    /// Save the key into the current macro list
    pub fn record(&mut self, key_to_record: KC, mods_to_record: &Modifiers) {
        if self.state == State::RecordInProgress {
            if let Some(entry) = self.macros.get_mut(&self.key_record) {
                entry.push((key_to_record, mods_to_record.clone())).ok();
            }
        }
    }

    pub fn up_led_status(&self, led_status: u8) -> u8 {
        match self.state {
            State::Inactive => led_status,
            State::RecordWaitKey => LED_DYNMAC_REC_WAIT,
            State::RecordInProgress => LED_DYNMAC_REC,
            State::GoWaitKey => LED_DYNMAC_GO_WAIT,
        }
    }
}

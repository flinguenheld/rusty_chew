use heapless::{Deque, FnvIndexMap, Vec};
use usbd_human_interface_device::device::mouse::WheelMouseReport;

use super::{
    keys::{Buffer, KC},
    modifiers::Modifiers,
    mouse::Mouse,
};
use crate::{
    hardware::{
        led::{LED_CAPLOCK, LED_LAYOUT_FN, LED_LAYOUT_FR, LED_LEADER_KEY},
        matrix::Matrix,
    },
    layouts::{COMBOS, LAYOUTS, LEADER_KEY_COMBINATIONS},
    options::{COMBO_TIME, HOLD_TIME, NB_KEYS},
};

// Remove pub --
#[derive(Clone, Copy)]
pub struct Key {
    pub index: usize,
    pub code: KC,
    pub ticks: u32,
}
impl Key {
    fn default() -> Self {
        Key {
            index: 0,
            code: KC::None,
            ticks: 0,
        }
    }
}

struct Layout {
    number: usize,
    index: usize,
    default: usize,
    dead: bool,
    dead_done: bool,
}
struct Leader {
    active: bool,
    buffer: Vec<KC, 3>,
}
struct DynMac {
    state: u8,
    key_index: KC,
    list: FnvIndexMap<KC, Vec<(KC, Modifiers), 64>, 32>,
}

/// This is the core of this keyboard,
/// The Run function proceeds all the keyboard hacks to fill the key buffer according to the LAYOUT.
pub struct Chew {
    layout: Layout,
    leader: Leader,
    dynmac: DynMac,
    mouse: Mouse,

    led_status: u8,

    matrix: Matrix,
    mods: Modifiers,
    homerow: Deque<Key, 5>,

    pre_pressed_keys: Vec<Key, NB_KEYS>,
    pressed_keys: Vec<Key, NB_KEYS>,
    // released_keys: Vec<usize, NB_KEYS>,
    last_key: Option<usize>,
    last_ticks: u32,
}

impl Chew {
    pub fn new(ticks: u32) -> Self {
        Chew {
            layout: Layout {
                number: 0,
                index: 0,
                default: 0,
                dead: false,
                dead_done: false,
            },
            leader: Leader {
                active: false,
                buffer: Vec::new(),
            },
            dynmac: DynMac {
                state: 0,
                key_index: KC::None,
                list: FnvIndexMap::new(),
            },
            mouse: Mouse::new(),
            led_status: 0,

            matrix: Matrix::new(),
            mods: Modifiers::new(),
            homerow: Deque::new(),

            pre_pressed_keys: Vec::new(),
            pressed_keys: Vec::new(),
            // released_keys: Vec::new(),
            last_key: None,
            last_ticks: ticks,
        }
    }

    pub fn update_matrix(&mut self, active_indexes: Vec<u8, 16>, ticks: u32) {
        self.matrix.update(active_indexes);

        // Clean --
        self.pre_pressed_keys
            .retain(|key| self.matrix.is_active(key.index) && key.code != KC::Done);
        self.pressed_keys
            .retain(|key| self.matrix.is_active(key.index) && key.code != KC::Done);

        // Updates --
        self.pre_pressed_keys
            .iter_mut()
            .chain(self.pressed_keys.iter_mut())
            .chain(self.homerow.iter_mut())
            .for_each(|key| {
                key.ticks += match self.last_ticks <= ticks {
                    true => ticks - self.last_ticks,
                    false => ticks + (u32::MAX - self.last_ticks),
                }
            });

        self.pre_pressed_keys.extend(
            self.matrix
                .freshly_pressed()
                .iter()
                .map(|v| Key {
                    index: *v,
                    ticks: 1,
                    ..Key::default()
                })
                .collect::<Vec<Key, 16>>(),
        );

        // Move pre-pressed keys into pressed keys --
        for key in self
            .pre_pressed_keys
            .iter_mut()
            .filter(|k| k.ticks > COMBO_TIME)
        {
            self.pressed_keys.push(*key).ok();
            key.code = KC::Done;
        }

        // Update mods status if released --
        self.mods.update_state(&self.pressed_keys);

        self.last_ticks = ticks;
    }

    pub fn run(
        &mut self,
        mut key_buffer: Buffer,
        mut mouse_report: WheelMouseReport,
        ticks: u32,
    ) -> (Buffer, WheelMouseReport, u8) {
        // Set new keys with the current layout -----------------------------------------
        for key in self
            .pre_pressed_keys
            .iter_mut()
            .filter(|k| k.code == KC::None)
        {
            key.code = LAYOUTS[self.layout.number][key.index];
        }

        // Combos -----------------------------------------------------------------------
        for (combo, new_key) in COMBOS.iter() {
            // Are these keys currently pressed ?
            if let Some(first) = self.pre_pressed_keys.iter().position(|k| match k.code {
                KC::HomeRow(_, c) => *c == combo[0],
                c => c == combo[0],
            }) {
                if let Some(second) = self.pre_pressed_keys.iter_mut().position(|k| match k.code {
                    KC::HomeRow(_, c) => *c == combo[1],
                    c => c == combo[1],
                }) {
                    // Remove pre-pressed & create the new pressed one
                    self.pre_pressed_keys[first].code = KC::Done;
                    self.pre_pressed_keys[second].code = KC::Done;

                    self.pressed_keys
                        .push(Key {
                            index: self.pre_pressed_keys[first].index,
                            code: *new_key,
                            ticks: COMBO_TIME,
                        })
                        .ok();
                }
            }
        }

        // Layout -----------------------------------------------------------------------
        if !(self.matrix.is_active(self.layout.index) || self.layout.dead && !self.layout.dead_done)
        {
            self.layout.number = self.layout.default;
            self.layout.dead = false;
        }

        for key in self.pressed_keys.iter_mut() {
            match key.code {
                KC::Layout(number) => {
                    // Allow this key to stay in key_pressed without being re-proceeded
                    key.code = KC::DoneButKeep;
                    self.layout.number = number;
                    self.layout.index = key.index;
                }
                KC::LaySet(number) => {
                    key.code = KC::Done;
                    self.layout.default = number;
                    self.layout.number = number;
                    self.layout.index = key.index;
                }
                KC::LayDead(number) => {
                    key.code = KC::DoneButKeep;
                    self.layout.number = number;
                    self.layout.index = key.index;

                    self.layout.dead = true;
                    self.layout.dead_done = false;
                }
                _ => {}
            }
        }

        // Leader key -------------------------------------------------------------------
        // Once activated, leave it with ESC or a wrong combination
        if let Some(leader) = self
            .pressed_keys
            .iter_mut()
            .find(|k| k.code == KC::LeaderKey)
        {
            self.leader.active = true;
            self.leader.buffer.clear();
            leader.code = KC::Done;
        }

        if self.leader.active {
            let mut success = None;
            for key in self.pressed_keys.iter_mut() {
                match key.code {
                    KC::Esc => {
                        self.leader.active = false;
                    }
                    KC::HomeRow(_, &k) | k
                        if (k >= KC::A && k <= KC::OE) || (k >= KC::Num0 && k <= KC::Tion) =>
                    {
                        self.leader.buffer.push(k).ok();
                        let temp_buffer: [KC; 3] = [
                            *self.leader.buffer.first().unwrap_or(&KC::None),
                            *self.leader.buffer.get(1).unwrap_or(&KC::None),
                            *self.leader.buffer.get(2).unwrap_or(&KC::None),
                        ];

                        if let Some((_, to_print)) = LEADER_KEY_COMBINATIONS
                            .iter()
                            .find(|(comb, _)| *comb == temp_buffer)
                        {
                            success = Some(to_print);
                            self.leader.active = false;
                        } else if !LEADER_KEY_COMBINATIONS
                            .iter()
                            .any(|(comb, _)| comb.starts_with(&self.leader.buffer))
                        {
                            self.leader.active = false;
                        }
                    }
                    _ => {}
                }

                // Deactivate all keys --
                key.code = KC::None;
            }

            if let Some(t) = success {
                self.pressed_keys
                    .push(Key {
                        index: usize::MAX,
                        code: *t,
                        ticks: COMBO_TIME,
                    })
                    .ok();
            }
        }

        // Modifiers --------------------------------------------------------------------
        // Regulars --
        self.pressed_keys
            .iter()
            .filter(|k| k.code >= KC::Alt && k.code <= KC::Sft)
            .for_each(|k| self.mods.set(k.code, k.index));

        // Caplock --
        if let Some(caplock) = self.pressed_keys.iter_mut().find(|k| k.code == KC::CapLock) {
            self.mods.set(KC::CapLock, 0);
            caplock.code = KC::Done;
        }

        // Homerows --
        while let Some(index) = self
            .pressed_keys
            .iter()
            .position(|k| matches!(k.code, KC::HomeRow(_, _)))
        {
            self.homerow
                .push_back(self.pressed_keys.swap_remove(index))
                .ok();
        }

        // Manage active homerows --
        // The first entry is always a homerow key
        if let Some(key) = self.homerow.front() {
            // First hold --
            // Set all homerows as held and print the regular keys
            if key.ticks >= HOLD_TIME {
                'hr: while let Some(mut popped_key) = self.homerow.pop_front() {
                    match popped_key.code {
                        KC::HomeRow(modifier, regular) => {
                            if popped_key.ticks >= HOLD_TIME {
                                self.mods.set(*modifier, popped_key.index);
                                popped_key.code = *modifier;

                                // Reintroduce the now-mod key
                                self.pressed_keys.push(popped_key).ok();

                            // Print home as Regular key if released
                            } else if !self.matrix.is_active(popped_key.index) {
                                key_buffer = regular.usb_code(key_buffer, &self.mods);
                                // TODO: Confirm that, is it necessary to update the key code ?
                                self.last_key = Some(popped_key.index);
                            } else {
                                // Specific case with two homerow pressed consecutively
                                // If the second is in an 'in-between' state, stop here to wait.
                                self.homerow.push_front(popped_key).ok();
                                break 'hr;
                            }
                        }
                        _ => {
                            // Non Homerow key
                            key_buffer = popped_key.code.usb_code(key_buffer, &self.mods);
                            self.last_key = Some(popped_key.index);
                        }
                    }
                }
            // First released bebore being held --
            // Print all of them with homerow pressed status
            } else if !self.matrix.is_active(key.index) {
                while let Some(popped_key) = self.homerow.pop_front() {
                    key_buffer = popped_key.code.usb_code(key_buffer, &self.mods);
                    self.last_key = Some(popped_key.index);
                }
            }
        }

        // Dynamic macros ---------------------------------------------------------------
        match self.dynmac.state {
            0 => {
                // Start a new record --
                if let Some(key) = self
                    .pressed_keys
                    .iter_mut()
                    .find(|k| k.code == KC::DynMacRecord)
                {
                    self.dynmac.state = 1;
                    key.code = KC::Done;

                // Go --
                } else if let Some(key) = self
                    .pressed_keys
                    .iter_mut()
                    .find(|k| k.code == KC::DynMacGo)
                {
                    self.dynmac.state = 10;
                    key.code = KC::Done;
                }
            }

            // Record active, now select the key to save it (erase if it already exists)
            1 => {
                if let Some(k) = self.pressed_keys.first_mut() {
                    self.dynmac.list.insert(k.code, Vec::new()).ok();
                    self.dynmac.key_index = k.code;
                    self.dynmac.state = 2;
                    k.code = KC::Done;
                }
            }

            // Stop recording
            2 => {
                if let Some(key) = self
                    .pre_pressed_keys
                    .iter_mut()
                    .find(|k| LAYOUTS[self.layout.number][k.index] == KC::DynMacRecord)
                {
                    self.dynmac.state = 0;
                    key.code = KC::Done;
                }
            }

            // Go active, now select the macro
            10 => {
                if let Some(k) = self.pressed_keys.first_mut() {
                    // Then fill the key buffer
                    if let Some(list) = self.dynmac.list.get(&k.code) {
                        let mut previous = KC::None;
                        for (k, m) in list.iter() {
                            if previous == *k {
                                // Add a break to allow twice same key
                                key_buffer = KC::None.usb_code(key_buffer, &self.mods);
                            }
                            key_buffer = k.usb_code(key_buffer, &m);
                            previous = *k;
                        }
                        key_buffer = KC::None.usb_code(key_buffer, &self.mods);
                    }

                    self.dynmac.state = 0;
                    k.code = KC::Done;
                }
            }

            _ => {}
        }

        // Regular keys -----------------------------------------------------------------
        if self.pressed_keys.iter().any(|k| k.code == KC::Esc) {
            self.mods.caplock = false;
        }

        for key in self
            .pressed_keys
            .iter_mut()
            .filter(|k| k.code > KC::DoneButKeep)
        {
            match key.code {
                k if (k >= KC::A && k <= KC::Tion || (k >= KC::MacroGit)) => {
                    if !self.homerow.is_empty() {
                        self.homerow.push_back(*key).ok();
                    } else {
                        key_buffer = k.usb_code(key_buffer, &self.mods);

                        // Dynamic macro record --
                        if self.dynmac.state == 2 {
                            if let Some(entry) = self.dynmac.list.get_mut(&self.dynmac.key_index) {
                                entry.push((key.code, self.mods.clone())).ok();
                            }
                        }
                    }

                    // No held with macros (They already add the NoEventIndicated)
                    if k < KC::ACircum {
                        self.last_key = Some(key.index);
                    }
                    key.code = KC::Done;
                    self.layout.dead = false;
                }

                // Mouse ----------------------------------------------------------------
                k if (k >= KC::MouseBtLeft && k <= KC::MouseBtRight) => {
                    self.mouse.active_button(&mut mouse_report, key);
                }
                k if (k >= KC::MouseSpeed1 && k <= KC::MouseSpeed4) => {
                    self.mouse.speed(key);
                    key.code = KC::DoneButKeep;
                }
                k if (k >= KC::MouseLeft && k <= KC::MouseRight) => {
                    self.mouse.movement(&mut mouse_report, k, ticks);
                }
                k if (k >= KC::MouseWheelLeft && k <= KC::MouseWheelRight) => {
                    self.mouse.scroll(&mut mouse_report, k, ticks);
                }

                _ => {}
            }
        }

        // Repetition -------------------------------------------------------------------
        if self.last_key.is_some_and(|index| {
            !(self.matrix.is_active(index) || (index == usize::MAX && self.mods.caplock))
        }) {
            // The potential still active mod(s) will set the value again.
            self.last_key = None;

            // End --
            if self.mods.active().is_empty() {
                key_buffer = KC::None.usb_code(key_buffer, &self.mods);
            }
        }

        // Add the active mods (useful for the real mouse) --
        if self.last_key.is_none() && !self.mods.active().is_empty() {
            for (key, index) in self.mods.active_kc().iter() {
                key_buffer = key.usb_code(key_buffer, &self.mods);
                self.last_key = Some(*index);
            }
        }

        // --
        self.mouse.release(&self.matrix, &mut mouse_report);

        // --
        self.led_status = match self.layout.number {
            4 => LED_LAYOUT_FR,
            5 => LED_LAYOUT_FN,
            _ => 0,
        };
        if self.leader.active {
            self.led_status = LED_LEADER_KEY;
        }
        if self.mods.caplock {
            self.led_status = LED_CAPLOCK;
        }
        match self.dynmac.state {
            1 => self.led_status = LED_LEADER_KEY,
            2 => self.led_status = LED_CAPLOCK,
            9 => self.led_status = LED_LAYOUT_FR,
            10 => self.led_status = LED_LAYOUT_FN,
            _ => {}
        }

        (key_buffer, mouse_report, self.led_status)
    }
}

use crate::{
    keys::{Buffer, KC},
    layouts::{COMBOS, LAYOUTS, LEADER_KEY_COMBINATIONS},
    utils::{
        led::{LED_LAYOUT_FR, LED_LEADER_KEY},
        matrix::Matrix,
        modifiers::Modifiers,
        options::{
            COMBO_TIME, HOLD_TIME, MOUSE_SPEED_1, MOUSE_SPEED_2, MOUSE_SPEED_3, MOUSE_SPEED_4,
            MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_1, SCROLL_TEMP_SPEED_2, SCROLL_TEMP_SPEED_3,
            SCROLL_TEMP_SPEED_4, SCROLL_TEMP_SPEED_DEFAULT,
        },
    },
};

use heapless::{Deque, FnvIndexMap, Vec};
use usbd_human_interface_device::device::mouse::WheelMouseReport;

// Remove pub --
#[derive(Clone)]
pub struct Key {
    pub index: usize,
    pub code: KC,
    pub ticks: u32,
    // pub done: bool,
}
impl Key {
    fn default() -> Self {
        Key {
            index: 0,
            code: KC::None,
            ticks: 0,
            // done: false,
        }
    }
}

struct Layout {
    number: usize,
    index: usize,
    dead: bool,
    dead_done: bool,
}
struct Leader {
    active: bool,
    buffer: Vec<KC, 3>,
}

/// This is the core of this keyboard,
/// The Run function proceeds all the keyboard hacks to fill the key buffer according
/// to the LAYOUT.
pub struct Chew {
    layout: Layout,
    led_status: u8,

    matrix: Matrix,
    mods: Modifiers,

    homerow: Deque<Key, 5>,

    // Allow to drastically slow down the wheel
    mouse_scroll_tempo: u32,

    leader: Leader,

    to_skip: Vec<(usize, u32), 10>,

    pre_pressed_keys: Vec<Key, 34>,
    pressed_keys: Vec<Key, 34>,
    released_keys: Vec<usize, 34>,

    last_ticks: u32,
    last_key: Option<usize>,
}

impl Chew {
    pub fn new(ticks: u32) -> Self {
        Chew {
            layout: Layout {
                number: 0,
                index: 0,
                dead: false,
                dead_done: false,
            },
            led_status: 0,

            matrix: Matrix::new(),
            mods: Modifiers::new(),

            homerow: Deque::new(),

            mouse_scroll_tempo: 0,

            leader: Leader {
                active: false,
                buffer: Vec::new(),
            },

            to_skip: Vec::new(),

            pre_pressed_keys: Vec::new(),
            pressed_keys: Vec::new(),
            released_keys: Vec::new(),

            last_ticks: ticks,
            last_key: None,
        }
    }

    pub fn update_matrix(&mut self, left: &Vec<u8, 8>, right: &Vec<u8, 8>, ticks: u32) {
        self.matrix
            .update_new(left.iter().chain(right.iter()).cloned().collect());

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
            self.pressed_keys.push(key.clone()).ok();
            key.code = KC::Done;
        }

        // Update mods status if released --
        self.mods.update_state(&self.pressed_keys);

        self.last_ticks = ticks;
    }

    pub fn is_last_key_active(&self, indexes: &[usize]) -> bool {
        // !indexes.is_empty() && indexes.iter().all(|i| self.matrix.cur[*i] > 0)
        !indexes.is_empty() && indexes.iter().all(|i| self.matrix.is_active(*i))
    }
    pub fn nb_active(&self) -> u32 {
        self.pressed_keys.len() as u32
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

        // Combos -------------------------------------------------------------------
        for (combo, new_key) in COMBOS.iter() {
            // Are these keys currently pressed ?
            if let Some(first) = self
                .pre_pressed_keys
                .iter()
                .position(|k| k.code == combo[0])
            {
                if let Some(second) = self
                    .pre_pressed_keys
                    .iter_mut()
                    .position(|k| k.code == combo[1])
                {
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
        // }

        // Layout -------------------------------------------------------------------
        if !self.matrix.is_active(self.layout.index)
            && (!self.layout.dead || (self.layout.dead && self.layout.dead_done))
        {
            self.layout.number = 0;
            self.layout.dead = false;
        }

        for key in self.pressed_keys.iter_mut() {
            match key.code {
                KC::Layout(number) => {
                    // Allow this key to stay in key_pressed without being re-proceeded
                    key.code = KC::LayoutDone;
                    self.layout.number = number;
                    self.layout.index = key.index;
                    self.layout.dead = false;
                }
                KC::LayDead(number) => {
                    key.code = KC::LayoutDone;
                    self.layout.number = number;
                    self.layout.index = key.index;
                    self.layout.dead = true;
                    self.layout.dead_done = false;
                }
                _ => {}
            }
        }

        // Leader key ---------------------------------------------------------------
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
            for key in self.pressed_keys.iter_mut() {
                match key.code {
                    KC::Esc => {
                        self.leader.active = false;
                    }
                    k if (k >= KC::A && k <= KC::OE)
                        || (k >= KC::Num0 && k <= KC::YDiaer)
                        || (k >= KC::HomeAltA && k <= KC::HomeSftR) =>
                    {
                        self.leader.buffer.push(k).ok();
                        let temp_buffer: [KC; 3] = [
                            *self.leader.buffer.get(0).unwrap_or(&KC::None),
                            *self.leader.buffer.get(1).unwrap_or(&KC::None),
                            *self.leader.buffer.get(2).unwrap_or(&KC::None),
                        ];

                        if let Some((_, to_print)) = LEADER_KEY_COMBINATIONS
                            .iter()
                            .find(|(comb, _)| *comb == temp_buffer)
                        {
                            key_buffer = to_print.usb_code(key_buffer, &self.mods);
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
        }

        // Modifiers ------------------------------------------------------------
        // Regulars --
        self.pressed_keys
            .iter()
            .filter(|k| k.code >= KC::Alt && k.code <= KC::Shift)
            .for_each(|k| self.mods.set(k.code, k.index));

        // Homerows --
        while let Some(index) = self
            .pressed_keys
            .iter()
            .position(|k| k.code >= KC::HomeAltA && k.code <= KC::HomeSftR)
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
                    if popped_key.code >= KC::HomeAltA && popped_key.code <= KC::HomeSftR {
                        if popped_key.ticks >= HOLD_TIME {
                            match popped_key.code {
                                KC::HomeAltA | KC::HomeAltU => {
                                    self.mods.set(KC::Alt, popped_key.index);
                                    popped_key.code = KC::Alt;
                                }
                                KC::HomeCtrlE | KC::HomeCtrlT => {
                                    self.mods.set(KC::Ctrl, popped_key.index);
                                    popped_key.code = KC::Ctrl;
                                }
                                KC::HomeGuiS | KC::HomeGuiI => {
                                    self.mods.set(KC::Gui, popped_key.index);
                                    popped_key.code = KC::Gui;
                                }
                                KC::HomeSftN | KC::HomeSftR => {
                                    self.mods.set(KC::Shift, popped_key.index);
                                    popped_key.code = KC::Shift;
                                }
                                _ => {}
                            }
                            // Reintroduce the now-mod key
                            self.pressed_keys.push(popped_key).ok();

                        // Print home as Regular key if released
                        } else if !self.matrix.is_active(popped_key.index) {
                            key_buffer = popped_key.code.usb_code(key_buffer, &self.mods);
                            // key_buffer = KC::None.usb_code(key_buffer, &self.mods);
                            self.last_key = Some(popped_key.index);
                        } else {
                            // Specific case with two homerow pressed consecutively
                            // If the second is in an 'in-between' state, stop here to wait.
                            self.homerow.push_front(popped_key).ok();
                            break 'hr;
                        }
                    } else {
                        // As regular key
                        key_buffer = popped_key.code.usb_code(key_buffer, &self.mods);
                        // key_buffer = KC::None.usb_code(key_buffer, &self.mods);
                        self.last_key = Some(popped_key.index);
                    }
                }
            // First released bebore being held --
            // Print all of them with homerow pressed status
            // } else if key.ticks < HOLD_TIME && !self.matrix.is_active(key.index) {
            } else if !self.matrix.is_active(key.index) {
                while let Some(popped_key) = self.homerow.pop_front() {
                    key_buffer = popped_key.code.usb_code(key_buffer, &self.mods);
                    // key_buffer = KC::None.usb_code(key_buffer, &self.mods);
                    self.last_key = Some(popped_key.index);
                }
            }
        }

        // Regular keys ---------------------------------------------------------
        for key in self
            .pressed_keys
            .iter_mut()
            .filter(|k| k.code > KC::LayoutDone)
        {
            match key.code {
                k if (k >= KC::A && k <= KC::Yen) => {
                    if !self.homerow.is_empty() {
                        self.homerow.push_back(key.clone()).ok();
                    } else {
                        key_buffer = k.usb_code(key_buffer, &self.mods);
                    }

                    self.last_key = Some(key.index);
                    key.code = KC::Done;
                    self.layout.dead = false;
                }

                // Mouse --------------------------------------------------------
                // k if (k >= KC::MouseBtLeft && k <= KC::MouseBtRight) => {
                //     if *mat_cur > 0 {
                //         mouse_report.buttons |= match k {
                //             KC::MouseBtLeft => 0x1,
                //             KC::MouseBtMiddle => 0x4,
                //             _ => 0x2,
                //         }
                //     } else {
                //         mouse_report.buttons &= match k {
                //             KC::MouseBtLeft => 0xFF - 0x1,
                //             KC::MouseBtMiddle => 0xFF - 0x4,
                //             _ => 0xFF - 0x2,
                //         }
                //     }
                // }
                // k if (k >= &KC::MouseLeft && k <= &KC::MouseWheelRight) => {
                //     self.mouse_scroll_tempo += 1;

                //     if *mat_cur > 0 {
                //         let (speed, (scroll_temp, scroll_speed)) = if let Some((key, _)) =
                //             LAYOUTS[self.layout.number]
                //                 .iter()
                //                 .zip(self.matrix.cur.iter())
                //                 .filter(|(k, m)| {
                //                     **k >= KC::MouseSpeed1 && **k <= KC::MouseSpeed4 && **m > 0
                //                 })
                //                 .last()
                //         {
                //             match key {
                //                 KC::MouseSpeed1 => (MOUSE_SPEED_1, SCROLL_TEMP_SPEED_1),
                //                 KC::MouseSpeed2 => (MOUSE_SPEED_2, SCROLL_TEMP_SPEED_2),
                //                 KC::MouseSpeed3 => (MOUSE_SPEED_3, SCROLL_TEMP_SPEED_3),
                //                 _ => (MOUSE_SPEED_4, SCROLL_TEMP_SPEED_4),
                //             }
                //         } else {
                //             (MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_DEFAULT)
                //         };

                //         if k <= &KC::MouseRight {
                //             mouse_report = k.usb_mouse_move(mouse_report, speed);
                //         } else if self.mouse_scroll_tempo >= scroll_temp {
                //             self.mouse_scroll_tempo = 0;
                //             mouse_report = k.usb_mouse_move(mouse_report, scroll_speed);
                //         }
                //     }
                // }
                _ => {}
            }
        }

        // for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[self.layout.number]
        //     .iter()
        //     .enumerate()
        //     .zip(self.matrix.prev.iter().zip(self.matrix.cur.iter()))
        //     .filter(|((index, _), (_, _))| {
        //         !self.to_skip.iter().any(|(i, _)| i == index) && !self.mods.is_active(*index)
        //     })
        // {
        //     match layout {
        //         k if (k >= &KC::A && k <= &KC::Yen) => {
        //             if *mat_prev < PRESSED_TIME && *mat_cur >= PRESSED_TIME {
        //                 self.layout.dead = false;

        //                 if self.homerow.is_empty() {
        //                     key_buffer = k.usb_code(key_buffer, &[index], &self.mods);
        //                 } else {
        //                     self.homerow.push_back(KeyIndex { key: *k, index }).ok();
        //                 }
        //             }
        //         }
        //         k if (k >= &KC::ACircum && k <= &KC::YDiaer) => {
        //             if *mat_prev < PRESSED_TIME && *mat_cur >= PRESSED_TIME {
        //                 self.layout.dead = false;

        //                 if self.homerow.is_empty() {
        //                     key_buffer = k.usb_code(key_buffer, &[], &self.mods);
        //                 } else {
        //                     self.homerow.push_back(KeyIndex { key: *k, index }).ok();
        //                 }
        //             }
        //         }

        //         // Mouse --------------------------------------------------------
        //         k if (k >= &KC::MouseBtLeft && k <= &KC::MouseBtRight) => {
        //             if *mat_cur > 0 {
        //                 mouse_report.buttons |= match k {
        //                     KC::MouseBtLeft => 0x1,
        //                     KC::MouseBtMiddle => 0x4,
        //                     _ => 0x2,
        //                 }
        //             } else {
        //                 mouse_report.buttons &= match k {
        //                     KC::MouseBtLeft => 0xFF - 0x1,
        //                     KC::MouseBtMiddle => 0xFF - 0x4,
        //                     _ => 0xFF - 0x2,
        //                 }
        //             }
        //         }
        //         k if (k >= &KC::MouseLeft && k <= &KC::MouseWheelRight) => {
        //             self.mouse_scroll_tempo += 1;

        //             if *mat_cur > 0 {
        //                 let (speed, (scroll_temp, scroll_speed)) = if let Some((key, _)) =
        //                     LAYOUTS[self.layout.number]
        //                         .iter()
        //                         .zip(self.matrix.cur.iter())
        //                         .filter(|(k, m)| {
        //                             **k >= KC::MouseSpeed1 && **k <= KC::MouseSpeed4 && **m > 0
        //                         })
        //                         .last()
        //                 {
        //                     match key {
        //                         KC::MouseSpeed1 => (MOUSE_SPEED_1, SCROLL_TEMP_SPEED_1),
        //                         KC::MouseSpeed2 => (MOUSE_SPEED_2, SCROLL_TEMP_SPEED_2),
        //                         KC::MouseSpeed3 => (MOUSE_SPEED_3, SCROLL_TEMP_SPEED_3),
        //                         _ => (MOUSE_SPEED_4, SCROLL_TEMP_SPEED_4),
        //                     }
        //                 } else {
        //                     (MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_DEFAULT)
        //                 };

        //                 if k <= &KC::MouseRight {
        //                     mouse_report = k.usb_mouse_move(mouse_report, speed);
        //                 } else if self.mouse_scroll_tempo >= scroll_temp {
        //                     self.mouse_scroll_tempo = 0;
        //                     mouse_report = k.usb_mouse_move(mouse_report, scroll_speed);
        //                 }
        //             }
        //         }

        //         _ => {}
        // }
        // }
        // }

        if self
            .last_key
            .is_some_and(|index| !self.matrix.is_active(index))
        {
            self.last_key = None;

            // End repetition --
            if self.mods.active().is_empty() {
                key_buffer = KC::None.usb_code(key_buffer, &self.mods);
            }
        }

        // Add the active mods (useful for the real mouse) --
        if self.last_key.is_none() && !self.mods.active().is_empty() {
            for k in self.mods.active_kc().iter() {
                key_buffer = k.0.usb_code(key_buffer, &self.mods);
                self.last_key = Some(k.1);
            }
        }

        // --
        self.led_status = 0;
        if self.layout.number == 4 {
            self.led_status = LED_LAYOUT_FR;
        }
        if self.leader.active {
            self.led_status = LED_LEADER_KEY;
        }

        (key_buffer, mouse_report, self.led_status)
    }
}

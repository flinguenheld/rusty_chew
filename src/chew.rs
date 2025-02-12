use crate::{
    keys::{Buffer, COMBOS, KC, LEADER_KEY_COMBINATIONS},
    layouts::LAYOUTS,
    utils::{
        led::{LED_LAYOUT_FR, LED_LEADER_KEY},
        matrix::Matrix,
        modifiers::Modifiers,
        options::{
            COMBO_TIME, HOLD_TIME, MOUSE_SPEED_1, MOUSE_SPEED_2, MOUSE_SPEED_3, MOUSE_SPEED_4,
            MOUSE_SPEED_DEFAULT, PRESSED_TIME, SCROLL_TEMP_SPEED_1, SCROLL_TEMP_SPEED_2,
            SCROLL_TEMP_SPEED_3, SCROLL_TEMP_SPEED_4, SCROLL_TEMP_SPEED_DEFAULT,
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
    pub done: bool,
}
impl Key {
    fn default() -> Self {
        Key {
            index: 0,
            code: KC::None,
            ticks: 0,
            done: false,
        }
    }
}

struct Layout {
    number: usize,
    index: usize,
    dead: bool,
    dead_done: bool,
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

    leader_key: bool,
    leader_buffer: Vec<KC, 3>,

    to_skip: Vec<(usize, u32), 10>,

    pub pressed_keys: Vec<Key, 34>,
    pub released_keys: Vec<usize, 34>,

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

            leader_key: false,
            leader_buffer: Vec::new(),

            to_skip: Vec::new(),

            pressed_keys: Vec::new(),
            released_keys: Vec::new(),

            last_ticks: ticks,
            last_key: None,
        }
    }

    pub fn update_matrix(&mut self, left: &Vec<u8, 8>, right: &Vec<u8, 8>, ticks: u32) {
        self.matrix
            .update_new(left.iter().chain(right.iter()).cloned().collect());

        self.pressed_keys
            .retain(|key| self.matrix.is_active(key.index) && !key.done);

        self.pressed_keys
            .iter_mut()
            .chain(self.homerow.iter_mut())
            .for_each(|key| {
                key.ticks += match self.last_ticks <= ticks {
                    true => ticks - self.last_ticks,
                    false => ticks + (u32::MAX - self.last_ticks),
                }
            });

        self.pressed_keys.extend(
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
        // if !self.pressed_keys.is_empty() || !self.homerow.is_empty() {
        if true {
            // if self.matrix.is_matrix_active()
            //     && (!self.pressed_keys.is_empty() || !self.homerow.is_empty())
            // {
            // Layout -------------------------------------------------------------------
            if self
                .pressed_keys
                .iter()
                .all(|k| k.index != self.layout.index)
                && ((self.layout.dead && self.layout.dead_done)
                    || (!self.layout.dead && self.layout.number != 0))
            {
                self.layout.number = 0;
                self.layout.dead = false;
            }

            for key in self.pressed_keys.iter_mut().filter(|k| k.code == KC::None) {
                match LAYOUTS[self.layout.number][key.index] {
                    KC::Layout(number) => {
                        key.code = LAYOUTS[self.layout.number][key.index];
                        self.layout.number = number;
                        self.layout.index = key.index;
                        self.layout.dead = false;
                    }
                    KC::LayDead(number) => {
                        key.code = LAYOUTS[self.layout.number][key.index];
                        self.layout.number = number;
                        self.layout.index = key.index;
                        self.layout.dead = true;
                        self.layout.dead_done = false;
                    }
                    _ => {}
                }
            }

            // Set new keys with the new layout -----------------------------------------
            for key in self.pressed_keys.iter_mut().filter(|k| k.code == KC::None) {
                key.code = LAYOUTS[self.layout.number][key.index];
            }

            // Combos -------------------------------------------------------------------
            // let active_keys: Vec<(usize, KC), 34> = LAYOUTS[self.layout.number]
            //     .iter()
            //     .enumerate()
            //     .zip(self.matrix.cur.iter())
            //     .filter(|(_, &mat)| mat > 0 && mat <= COMBO_TIME)
            //     .map(|((index, key), _)| (index, *key))
            //     .collect();

            // for (combo, key) in COMBOS.iter() {
            //     let index_0 =
            //         active_keys
            //             .iter()
            //             .find_map(|(i, k)| if *k == combo[0] { Some(*i) } else { None });
            //     let index_1 =
            //         active_keys
            //             .iter()
            //             .find_map(|(i, k)| if *k == combo[1] { Some(*i) } else { None });

            //     if index_0.is_some() && index_1.is_some() {
            //         self.to_skip
            //             .push((index_0.unwrap(), ticks.wrapping_add(COMBO_TIME)))
            //             .ok();
            //         self.to_skip
            //             .push((index_1.unwrap(), ticks.wrapping_add(COMBO_TIME)))
            //             .ok();

            //         match *key {
            //             k if k >= KC::A && k <= KC::YDiaer => {
            //                 key_buffer = k.usb_code(
            //                     key_buffer,
            //                     &[index_0.unwrap(), index_1.unwrap()],
            //                     &self.mods,
            //                 );
            //                 // return (key_buffer, mouse_report, self.led_status);
            //             }

            //             KC::Layout(number) => {
            //                 self.layout.number = number;
            //             }
            //             _ => {}
            //         }
            //     }
            // }

            // Layouts ------------------------------------------------------------------
            // if !self.layout.dead {
            //     if self.matrix.cur[self.layout.matrix_index] == 0 {
            //         self.layout.number = 0;
            //     }

            //     for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[self.layout.number]
            //         .iter()
            //         .enumerate()
            //         .zip(self.matrix.prev.iter().zip(self.matrix.cur.iter()))
            //     {
            //         match layout {
            //             KC::Layout(number) => {
            //                 if *mat_cur > 0 {
            //                     self.layout.number = *number;
            //                     self.layout.matrix_index = index;
            //                     self.layout.dead = false;
            //                 }
            //             }
            //             KC::LayDead(number) => {
            //                 if *mat_cur > 0 {
            //                     self.layout.number = *number;
            //                     self.layout.matrix_index = index;

            //                     if *mat_prev == 0 {
            //                         self.layout.dead = true;
            //                         self.to_skip
            //                             .push((index, ticks.wrapping_add(PRESSED_TIME)))
            //                             .ok();
            //                     } else {
            //                         self.layout.dead = false;
            //                     }
            //                 }
            //             }
            //             _ => {}
            //         }
            //     }
            // }

            // // Leader key ---------------------------------------------------------------
            // // Once activated, leave it with ESC or a wrong combination
            // if self.leader_key
            //     || LAYOUTS[self.layout.number]
            //         .iter()
            //         .zip(self.matrix.cur.iter())
            //         .filter(|(&k, &m)| k == KC::LeaderKey && m > 0)
            //         .count()
            //         > 0
            // {
            //     self.leader_key = true;

            //     for ((index, &layout), (mat_prev, mat_cur)) in LAYOUTS[self.layout.number]
            //         .iter()
            //         .enumerate()
            //         .zip(self.matrix.prev.iter().zip(self.matrix.cur.iter()))
            //         .filter(|((_, &k), (_, _))| {
            //             (k >= KC::A && k <= KC::YDiaer) || (k >= KC::HomeAltA && k <= KC::HomeSftR)
            //         })
            //     {
            //         if *mat_prev == 0 && *mat_cur > 0 {
            //             self.to_skip
            //                 .push((index, ticks.wrapping_add(PRESSED_TIME)))
            //                 .ok();

            //             self.leader_buffer.push(layout).ok();
            //             let mut temp_buff = [KC::None; 3];
            //             for (i, v) in self.leader_buffer.iter().enumerate() {
            //                 temp_buff[i] = *v;
            //             }

            //             if let Some(i) = LEADER_KEY_COMBINATIONS
            //                 .iter()
            //                 .position(|comb| comb.0 == temp_buff)
            //             {
            //                 key_buffer =
            //                     LEADER_KEY_COMBINATIONS[i]
            //                         .1
            //                         .usb_code(key_buffer, &[], &self.mods);
            //             } else if layout != KC::Esc
            //                 && LEADER_KEY_COMBINATIONS
            //                     .iter()
            //                     .filter(|comb| comb.0.starts_with(&self.leader_buffer))
            //                     .count()
            //                     > 0
            //             {
            //                 continue;
            //             }

            //             self.leader_key = false;
            //             self.leader_buffer.clear();
            //             break;
            //         }
            //     }
            // } else {

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

            // CLEAN IT  ---------------------------------------------------------
            // CLEAN IT  ---------------------------------------------------------

            //     // Modifiers ------------------------------------------------------------
            //     LAYOUTS[self.layout.number]
            //         .iter()
            //         .zip(self.matrix.cur.iter())
            //         .enumerate()
            //         .filter(|(_, (&la, &mc))| mc > 0 && (la >= KC::Alt && la <= KC::Shift))
            //         .for_each(|(index, (layout, _))| match layout {
            //             KC::Alt => self.mods.alt = (true, index),
            //             KC::Altgr => self.mods.alt_gr = (true, index),
            //             KC::Ctrl => self.mods.ctrl = (true, index),
            //             KC::Gui => self.mods.gui = (true, index),
            //             _ => self.mods.shift = (true, index),
            //         });

            //     self.mods.deactivate_released(&self.matrix.cur);

            //     // Homerows -------------------------------------------------------------
            //     // Get and add new active ones --
            //     for (((index, &key), _), _) in LAYOUTS[self.layout.number]
            //         .iter()
            //         .enumerate()
            //         .zip(self.matrix.prev.iter())
            //         .zip(self.matrix.cur.iter())
            //         .filter(|(((_, &key), &mat_prev), &mat_cur)| {
            //             (key >= KC::HomeAltA && key <= KC::HomeSftR)
            //                 && (mat_prev == 0)
            //                 && mat_cur > 0
            //         })
            //     {
            //         self.homerow.push_back(KeyIndex { key, index }).ok();
            //     }

            //     // Manage active homerows --
            //     // The first entry is always a homerow key
            //     if let Some(key_index) = self.homerow.front() {
            //         // First hold --
            //         // Set all homerows as held and print the regular keys
            //         if self.matrix.cur[key_index.index] > HOLD_TIME {
            //             'hr: while let Some(ki) = self.homerow.pop_front() {
            //                 if ki.key >= KC::HomeAltA && ki.key <= KC::HomeSftR {
            //                     if self.matrix.cur[ki.index] >= HOLD_TIME {
            //                         match ki.key {
            //                             KC::HomeAltA | KC::HomeAltU => {
            //                                 self.mods.alt = (true, ki.index)
            //                             }
            //                             KC::HomeCtrlE | KC::HomeCtrlT => {
            //                                 self.mods.ctrl = (true, ki.index)
            //                             }
            //                             KC::HomeGuiS | KC::HomeGuiI => {
            //                                 self.mods.gui = (true, ki.index)
            //                             }
            //                             KC::HomeSftN | KC::HomeSftR => {
            //                                 self.mods.shift = (true, ki.index)
            //                             }
            //                             _ => {}
            //                         }
            //                     } else if self.matrix.cur[ki.index] == 0 {
            //                         key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
            //                     } else {
            //                         // Specific case with two homerow pressed consecutively
            //                         // If the second is in an 'in-between' state, stop here and break.
            //                         self.homerow.push_front(ki).ok();
            //                         break 'hr;
            //                     }
            //                 } else {
            //                     key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
            //                 }
            //             }
            //         // First released bebore being held --
            //         // Print all of them with homerow pressed status
            //         } else if self.matrix.prev[key_index.index] < HOLD_TIME
            //             && self.matrix.cur[key_index.index] == 0
            //         {
            //             while let Some(ki) = self.homerow.pop_front() {
            //                 key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
            //             }
            //         }
            //     }
            //     // CLEAN IT  ---------------------------------------------------------
            //     // CLEAN IT  ---------------------------------------------------------

            //     // To allow held repetition, do it only if there no other active key
            //     if !LAYOUTS[self.layout.number]
            //         .iter()
            //         .zip(self.matrix.cur.iter())
            //         .any(|(&k, &m)| m > 0 && k >= KC::A && k <= KC::YDiaer)
            //     {
            //         if self.mods.alt.0 {
            //             key_buffer = KC::Alt.usb_code(key_buffer, &[self.mods.alt.1], &self.mods);
            //         }
            //         if self.mods.alt_gr.0 {
            //             key_buffer =
            //                 KC::Altgr.usb_code(key_buffer, &[self.mods.alt_gr.1], &self.mods);
            //         }
            //         if self.mods.ctrl.0 {
            //             key_buffer = KC::Ctrl.usb_code(key_buffer, &[self.mods.ctrl.1], &self.mods);
            //         }
            //         if self.mods.gui.0 {
            //             key_buffer = KC::Gui.usb_code(key_buffer, &[self.mods.gui.1], &self.mods);
            //         }
            //         if self.mods.shift.0 {
            //             key_buffer =
            //                 KC::Shift.usb_code(key_buffer, &[self.mods.shift.1], &self.mods);
            //         }
            //     }
            //     // CLEAN IT  ---------------------------------------------------------
            //     // CLEAN IT  ---------------------------------------------------------

            // Regular keys ---------------------------------------------------------
            // Filtering mods prevents error with layers

            for key in self.pressed_keys.iter_mut().filter(|k| !k.done) {
                match key.code {
                    k if (k >= KC::A && k <= KC::Yen) => {
                        if !self.homerow.is_empty() {
                            self.homerow.push_back(key.clone()).ok();
                        } else {
                            key_buffer = k.usb_code(key_buffer, &self.mods);
                        }

                        self.last_key = Some(key.index);
                        key.done = true;
                        self.layout.dead = false;
                    }
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
        }

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
        if self.leader_key {
            self.led_status = LED_LEADER_KEY;
        }

        (key_buffer, mouse_report, self.led_status)
    }
}

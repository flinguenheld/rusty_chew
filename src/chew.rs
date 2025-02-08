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

use heapless::{Deque, Vec};
use usbd_human_interface_device::device::mouse::WheelMouseReport;

struct Layout {
    number: usize,
    matrix_index: usize,
    dead: bool,
}

struct KeyIndex {
    key: KC,
    index: usize,
}

/// This is the core of this keyboard,
/// The Run function proceeds all the keyboard hacks to fill the key buffer according
/// to the LAYOUT.
pub struct Chew {
    layout: Layout,
    led_status: u8,

    matrix: Matrix,
    mods: Modifiers,

    homerow: Deque<KeyIndex, 5>,

    // Allow to drastically slow down the wheel
    mouse_scroll_tempo: u32,

    leader_key: bool,
    leader_buffer: Vec<KC, 3>,

    to_skip: Vec<(KC, u32), 10>,
}

impl Chew {
    pub fn new(ticks: u32) -> Self {
        Chew {
            layout: Layout {
                number: 0,
                matrix_index: 0,
                dead: false,
            },
            led_status: 0,

            matrix: Matrix::new(ticks),
            mods: Modifiers::new(),

            homerow: Deque::new(),

            mouse_scroll_tempo: 0,

            leader_key: false,
            leader_buffer: Vec::new(),

            to_skip: Vec::new(),
        }
    }

    pub fn update_matrix(&mut self, left: &Vec<u8, 8>, right: &Vec<u8, 8>, ticks: u32) {
        self.matrix
            .update(left.iter().chain(right.iter()).cloned().collect(), ticks);
    }

    pub fn is_last_key_active(&self, indexes: &[usize]) -> bool {
        !indexes.is_empty() && indexes.iter().all(|i| self.matrix.cur[*i] > 0)
    }

    pub fn run(
        &mut self,
        mut key_buffer: Buffer,
        mut mouse_report: WheelMouseReport,
        ticks: u32,
    ) -> (Buffer, WheelMouseReport, u8) {
        if self.matrix.prev != self.matrix.cur {
            // Combos -------------------------------------------------------------------
            let active_keys: Vec<(usize, KC), 34> = LAYOUTS[self.layout.number]
                .iter()
                .enumerate()
                .zip(self.matrix.cur.iter())
                .filter(|(_, &mat)| mat > 0 && mat <= COMBO_TIME)
                .map(|((index, key), _)| (index, *key))
                .collect();

            for (combo, key) in COMBOS.iter() {
                let index_0 =
                    active_keys
                        .iter()
                        .find_map(|(i, k)| if *k == combo[0] { Some(*i) } else { None });
                let index_1 =
                    active_keys
                        .iter()
                        .find_map(|(i, k)| if *k == combo[1] { Some(*i) } else { None });

                if index_0.is_some() && index_1.is_some() {
                    if *key >= KC::A && *key <= KC::YDiaer {
                        self.to_skip
                            .push((combo[0], ticks.wrapping_add(COMBO_TIME)))
                            .ok();
                        self.to_skip
                            .push((combo[1], ticks.wrapping_add(COMBO_TIME)))
                            .ok();

                        key_buffer = key.usb_code(
                            key_buffer,
                            &[index_0.unwrap(), index_1.unwrap()],
                            &self.mods,
                        );
                        return (key_buffer, mouse_report, self.led_status);
                    } else if *key >= KC::Layout(0) && *key <= KC::LayDead(0) {
                        // KC::F.usb_code(&self.mods, &mut key_buffer);
                        return (key_buffer, mouse_report, self.led_status);
                    }
                }
            }

            // Layouts ------------------------------------------------------------------
            if !self.layout.dead {
                if self.matrix.cur[self.layout.matrix_index] == 0 {
                    self.layout.number = 0;
                }

                for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[self.layout.number]
                    .iter()
                    .enumerate()
                    .zip(self.matrix.prev.iter().zip(self.matrix.cur.iter()))
                {
                    match layout {
                        KC::Layout(number) => {
                            if *mat_cur > 0 {
                                self.layout.number = *number;
                                self.layout.matrix_index = index;
                                self.layout.dead = false;
                            }
                        }
                        KC::LayDead(number) => {
                            if *mat_cur > 0 {
                                self.to_skip
                                    .push((*layout, ticks.wrapping_add(PRESSED_TIME)))
                                    .ok();

                                self.layout.number = *number;
                                self.layout.matrix_index = index;

                                if *mat_prev == 0 {
                                    // Set and return to avoid its own key pressed
                                    self.layout.dead = true;
                                    return (key_buffer, mouse_report, self.led_status);
                                } else {
                                    self.layout.dead = false;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Leader key ---------------------------------------------------------------
            // Once activated, leave it with ESC or a wrong combination
            if self.leader_key
                || LAYOUTS[self.layout.number]
                    .iter()
                    .zip(self.matrix.cur.iter())
                    .filter(|(&k, &m)| k == KC::LeaderKey && m > 0)
                    .count()
                    > 0
            {
                self.leader_key = true;

                for ((&layout, mat_prev), mat_cur) in LAYOUTS[self.layout.number]
                    .iter()
                    .zip(self.matrix.prev.iter())
                    .zip(self.matrix.cur.iter())
                    .filter(|((&k, _), _)| {
                        (k >= KC::A && k <= KC::YDiaer) || (k >= KC::HomeAltA && k <= KC::HomeSftR)
                    })
                {
                    if *mat_prev == 0 && *mat_cur > 0 {
                        self.to_skip
                            .push((layout, ticks.wrapping_add(PRESSED_TIME)))
                            .ok();

                        self.leader_buffer.push(layout).ok();
                        let mut temp_buff = [KC::None; 3];
                        for (i, v) in self.leader_buffer.iter().enumerate() {
                            temp_buff[i] = *v;
                        }

                        if let Some(i) = LEADER_KEY_COMBINATIONS
                            .iter()
                            .position(|comb| comb.0 == temp_buff)
                        {
                            key_buffer =
                                LEADER_KEY_COMBINATIONS[i]
                                    .1
                                    .usb_code(key_buffer, &[], &self.mods);
                        } else if layout != KC::Esc
                            && LEADER_KEY_COMBINATIONS
                                .iter()
                                .filter(|comb| comb.0.starts_with(&self.leader_buffer))
                                .count()
                                > 0
                        {
                            continue;
                        }

                        self.leader_key = false;
                        self.leader_buffer.clear();
                        break;
                    }
                }
            } else {
                // Modifiers ------------------------------------------------------------
                LAYOUTS[self.layout.number]
                    .iter()
                    .zip(self.matrix.cur.iter())
                    .enumerate()
                    .filter(|(_, (&la, &mc))| mc > 0 && (la >= KC::Alt && la <= KC::Shift))
                    .for_each(|(index, (layout, _))| match layout {
                        KC::Alt => self.mods.alt = (true, index),
                        KC::Altgr => self.mods.alt_gr = (true, index),
                        KC::Ctrl => self.mods.ctrl = (true, index),
                        KC::Gui => self.mods.gui = (true, index),
                        _ => self.mods.shift = (true, index),
                    });

                self.mods.deactivate_released(&self.matrix.cur);

                // Homerows -------------------------------------------------------------
                // Get and add new active ones --
                for (((index, &key), _), _) in LAYOUTS[self.layout.number]
                    .iter()
                    .enumerate()
                    .zip(self.matrix.prev.iter())
                    .zip(self.matrix.cur.iter())
                    .filter(|(((_, &key), &mat_prev), &mat_cur)| {
                        (key >= KC::HomeAltA && key <= KC::HomeSftR)
                            && (mat_prev == 0)
                            && mat_cur > 0
                    })
                {
                    self.homerow.push_back(KeyIndex { key, index }).ok();
                }

                // Manage active homerows --
                // The first entry is always a homerow key
                if let Some(key_index) = self.homerow.front() {
                    // First hold --
                    // Set all homerows as held and print the regular keys
                    if self.matrix.cur[key_index.index] > HOLD_TIME {
                        'hr: while let Some(ki) = self.homerow.pop_front() {
                            if ki.key >= KC::HomeAltA && ki.key <= KC::HomeSftR {
                                if self.matrix.cur[ki.index] >= HOLD_TIME {
                                    match ki.key {
                                        KC::HomeAltA | KC::HomeAltU => {
                                            self.mods.alt = (true, ki.index)
                                        }
                                        KC::HomeCtrlE | KC::HomeCtrlT => {
                                            self.mods.ctrl = (true, ki.index)
                                        }
                                        KC::HomeGuiS | KC::HomeGuiI => {
                                            self.mods.gui = (true, ki.index)
                                        }
                                        KC::HomeSftN | KC::HomeSftR => {
                                            self.mods.shift = (true, ki.index)
                                        }
                                        _ => {}
                                    }
                                } else if self.matrix.cur[ki.index] == 0 {
                                    key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
                                } else {
                                    // Specific case with two homerow pressed consecutively
                                    // If the second is in an 'in-between' state, stop here and break.
                                    self.homerow.push_front(ki).ok();
                                    break 'hr;
                                }
                            } else {
                                key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
                            }
                        }
                    // First released bebore being held --
                    // Print all of them with homerow pressed status
                    } else if self.matrix.prev[key_index.index] < HOLD_TIME
                        && self.matrix.cur[key_index.index] == 0
                    {
                        while let Some(ki) = self.homerow.pop_front() {
                            key_buffer = ki.key.usb_code(key_buffer, &[], &self.mods);
                        }
                    }
                }
                // CLEAN IT  ---------------------------------------------------------
                // CLEAN IT  ---------------------------------------------------------

                // To allow held repetition, do it only if there no other active key
                if !LAYOUTS[self.layout.number]
                    .iter()
                    .zip(self.matrix.cur.iter())
                    .any(|(&k, &m)| m > 0 && k >= KC::A && k <= KC::YDiaer)
                {
                    if self.mods.alt.0 {
                        key_buffer = KC::Alt.usb_code(key_buffer, &[self.mods.alt.1], &self.mods);
                    }
                    if self.mods.alt_gr.0 {
                        key_buffer =
                            KC::Altgr.usb_code(key_buffer, &[self.mods.alt_gr.1], &self.mods);
                    }
                    if self.mods.ctrl.0 {
                        key_buffer = KC::Ctrl.usb_code(key_buffer, &[self.mods.ctrl.1], &self.mods);
                    }
                    if self.mods.gui.0 {
                        key_buffer = KC::Gui.usb_code(key_buffer, &[self.mods.gui.1], &self.mods);
                    }
                    if self.mods.shift.0 {
                        key_buffer =
                            KC::Shift.usb_code(key_buffer, &[self.mods.shift.1], &self.mods);
                    }
                }
                // CLEAN IT  ---------------------------------------------------------
                // CLEAN IT  ---------------------------------------------------------

                // Regular keys ---------------------------------------------------------
                // Filtering mods prevents error with layers
                for (((index, layout), mat_prev), mat_cur) in LAYOUTS[self.layout.number]
                    .iter()
                    .enumerate()
                    .zip(self.matrix.prev.iter())
                    .zip(self.matrix.cur.iter())
                    .filter(|(((index, &key), _), _)| {
                        !self.to_skip.iter().any(|(k, _)| *k == key) && !self.mods.is_active(*index)
                    })
                {
                    match layout {
                        k if (k >= &KC::A && k <= &KC::Yen) => {
                            if *mat_prev < PRESSED_TIME && *mat_cur >= PRESSED_TIME {
                                self.layout.dead = false;

                                if self.homerow.is_empty() {
                                    key_buffer = k.usb_code(key_buffer, &[index], &self.mods);
                                } else {
                                    self.homerow.push_back(KeyIndex { key: *k, index }).ok();
                                }
                            }
                        }
                        k if (k >= &KC::ACircum && k <= &KC::YDiaer) => {
                            if *mat_prev < PRESSED_TIME && *mat_cur >= PRESSED_TIME {
                                self.layout.dead = false;

                                if self.homerow.is_empty() {
                                    key_buffer = k.usb_code(key_buffer, &[], &self.mods);
                                } else {
                                    self.homerow.push_back(KeyIndex { key: *k, index }).ok();
                                }
                            }
                        }

                        // Mouse --------------------------------------------------------
                        k if (k >= &KC::MouseBtLeft && k <= &KC::MouseBtRight) => {
                            if *mat_cur > 0 {
                                mouse_report.buttons |= match k {
                                    KC::MouseBtLeft => 0x1,
                                    KC::MouseBtMiddle => 0x4,
                                    _ => 0x2,
                                }
                            } else {
                                mouse_report.buttons &= match k {
                                    KC::MouseBtLeft => 0xFF - 0x1,
                                    KC::MouseBtMiddle => 0xFF - 0x4,
                                    _ => 0xFF - 0x2,
                                }
                            }
                        }
                        k if (k >= &KC::MouseLeft && k <= &KC::MouseWheelRight) => {
                            self.mouse_scroll_tempo += 1;

                            if *mat_cur > 0 {
                                let (speed, (scroll_temp, scroll_speed)) = if let Some((key, _)) =
                                    LAYOUTS[self.layout.number]
                                        .iter()
                                        .zip(self.matrix.cur.iter())
                                        .filter(|(k, m)| {
                                            **k >= KC::MouseSpeed1
                                                && **k <= KC::MouseSpeed4
                                                && **m > 0
                                        })
                                        .last()
                                {
                                    match key {
                                        KC::MouseSpeed1 => (MOUSE_SPEED_1, SCROLL_TEMP_SPEED_1),
                                        KC::MouseSpeed2 => (MOUSE_SPEED_2, SCROLL_TEMP_SPEED_2),
                                        KC::MouseSpeed3 => (MOUSE_SPEED_3, SCROLL_TEMP_SPEED_3),
                                        _ => (MOUSE_SPEED_4, SCROLL_TEMP_SPEED_4),
                                    }
                                } else {
                                    (MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_DEFAULT)
                                };

                                if k <= &KC::MouseRight {
                                    mouse_report = k.usb_mouse_move(mouse_report, speed);
                                } else if self.mouse_scroll_tempo >= scroll_temp {
                                    self.mouse_scroll_tempo = 0;
                                    mouse_report = k.usb_mouse_move(mouse_report, scroll_speed);
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }

            // if self.matrix.cur.iter().all(|m| *m == 0) {
            //     key_buffer = KC::None.usb_code(key_buffer, &self.mods);
            // }
        }

        self.to_skip.retain(|(_, t)| *t > ticks);

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

use crate::{
    keys::{Lay, KC, LEADER_KEY_COMBINATIONS},
    layouts::LAYOUTS,
    utils::{
        led::{LED_LAYOUT_FR, LED_LEADER_KEY},
        matrix::Matrix,
        modifiers::{self, Modifiers},
        options::{
            BUFFER_LENGTH, HOLD_TIME, MOUSE_SPEED_1, MOUSE_SPEED_2, MOUSE_SPEED_3, MOUSE_SPEED_4,
            MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_1, SCROLL_TEMP_SPEED_2, SCROLL_TEMP_SPEED_3,
            SCROLL_TEMP_SPEED_4, SCROLL_TEMP_SPEED_DEFAULT,
        },
    },
};

use heapless::{Deque, FnvIndexSet, Vec};
use usbd_human_interface_device::{device::mouse::WheelMouseReport, page::Keyboard};

const NB_LAYOUTS: usize = LAYOUTS.len();

struct KeyIndex {
    key: KC,
    index: usize,
}

/// This is the core of this keyboard,
/// The Run function proceeds all the keyboard hacks to fill the key buffer according
/// to the LAYOUT.
pub struct Chew {
    layouts: Vec<Lay, NB_LAYOUTS>,
    current_layout: usize,
    led_status: u8,

    matrix: Matrix,
    mods: Modifiers,
    homerow_history: FnvIndexSet<usize, 8>,

    homerow: Deque<KeyIndex, 10>,

    // Allow to drastically slow down the wheel
    mouse_scroll_tempo: u32,

    leader_key: bool,
    leader_buffer: Vec<KC, 3>,

    last_index_printed: usize,
}

impl Chew {
    pub fn new(ticks: u32) -> Self {
        Chew {
            layouts: Vec::new(),
            current_layout: 0,
            led_status: 0,

            matrix: Matrix::new(ticks),
            mods: Modifiers::new(),
            homerow_history: FnvIndexSet::new(),

            homerow: Deque::new(),

            mouse_scroll_tempo: 0,

            leader_key: false,
            leader_buffer: Vec::new(),

            last_index_printed: usize::MAX,
        }
    }

    pub fn update_matrix(&mut self, left: &Vec<u8, 8>, right: &Vec<u8, 8>, ticks: u32) {
        self.matrix
            .update(left.iter().chain(right.iter()).cloned().collect(), ticks);
    }

    pub fn run(
        &mut self,
        mut key_buffer: Deque<[Keyboard; 6], BUFFER_LENGTH>,
        mut mouse_report: WheelMouseReport,
    ) -> (Deque<[Keyboard; 6], BUFFER_LENGTH>, WheelMouseReport, u8) {
        if self.matrix.prev != self.matrix.cur {
            // Layouts ------------------------------------------------------------------
            match self.layouts.last().unwrap_or(&Lay::Pressed(0, 0)) {
                Lay::Dead(_, _, _) => {}
                _ => {
                    for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[self.current_layout]
                        .iter()
                        .enumerate()
                        .zip(self.matrix.prev.iter().zip(self.matrix.cur.iter()))
                    {
                        match layout {
                            KC::Layout(number) => {
                                if *mat_cur > 0 {
                                    self.layouts.push(Lay::Pressed(*number, index)).ok();
                                    // break;
                                }
                            }
                            KC::LayDead(number) => {
                                if *mat_prev == 0 && *mat_cur > 0 {
                                    self.layouts.push(Lay::Dead(*number, index, false)).ok();

                                    // Mandatorily jump to avoid its own key pressed
                                    return (key_buffer, mouse_report, self.led_status);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            self.current_layout = match self.layouts.last().unwrap_or(&Lay::Pressed(0, 0)) {
                Lay::Pressed(number, _) => *number,
                Lay::Dead(number, _, _) => *number,
            };

            // Leader key ---------------------------------------------------------------
            // Once activated, leave it with ESC or a wrong combination
            if self.leader_key
                || LAYOUTS[self.current_layout]
                    .iter()
                    .zip(self.matrix.cur.iter())
                    .filter(|(&k, &m)| k == KC::LeaderKey && m > 0)
                    .count()
                    > 0
            {
                self.leader_key = true;

                for ((&layout, mat_prev), mat_cur) in LAYOUTS[self.current_layout]
                    .iter()
                    .zip(self.matrix.prev.iter())
                    .zip(self.matrix.cur.iter())
                    .filter(|((&k, _), _)| {
                        (k >= KC::A && k <= KC::YDiaer) || (k >= KC::HomeAltA && k <= KC::HomeSftR)
                    })
                {
                    if *mat_prev == 0 && *mat_cur > 0 {
                        self.leader_buffer.push(layout).ok();
                        let mut temp_buff = [KC::None; 3];
                        for (i, v) in self.leader_buffer.iter().enumerate() {
                            temp_buff[i] = *v;
                        }

                        if let Some(i) = LEADER_KEY_COMBINATIONS
                            .iter()
                            .position(|comb| comb.0 == temp_buff)
                        {
                            LEADER_KEY_COMBINATIONS[i]
                                .1
                                .usb_code(&self.mods, &mut key_buffer);
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
                LAYOUTS[self.current_layout]
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

                // TODO still usefull ??
                self.mods.update_states(&self.matrix.cur);

                // Homerows -------------------------------------------------------------

                // Get and add new active homerow keys --
                for (((index, &key), _), _) in LAYOUTS[self.current_layout]
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
                if let Some(key_index) = self.homerow.front() {
                    // TODO specific case with two homerow in the same time ?
                    // First hold --
                    // Set all homerows as held and print the regular keys
                    if self.matrix.cur[key_index.index] > HOLD_TIME {
                        while let Some(ki) = self.homerow.pop_front() {
                            if ki.key >= KC::HomeAltA && ki.key <= KC::HomeSftR {
                                match ki.key {
                                    KC::HomeAltA | KC::HomeAltU => self.mods.alt = (true, ki.index),
                                    KC::HomeCtrlE | KC::HomeCtrlT => {
                                        self.mods.ctrl = (true, ki.index)
                                    }
                                    KC::HomeGuiS | KC::HomeGuiI => self.mods.gui = (true, ki.index),
                                    KC::HomeSftN | KC::HomeSftR => {
                                        self.mods.shift = (true, ki.index)
                                    }
                                    _ => {}
                                }
                            } else {
                                ki.key.usb_code(&self.mods, &mut key_buffer);
                                self.last_index_printed = ki.index;
                            }
                        }
                    // First released bebore being held --
                    // Print all of them with homerow pressed status
                    } else if self.matrix.prev[key_index.index] < HOLD_TIME
                        && self.matrix.cur[key_index.index] == 0
                    {
                        while let Some(ki) = self.homerow.pop_front() {
                            ki.key.usb_code(&self.mods, &mut key_buffer);
                            self.last_index_printed = ki.index;
                        }
                    }
                }

                // Regular keys ---------------------------------------------------------
                // Filter mods prevents error with layers
                for (((index, layout), mat_prev), mat_cur) in LAYOUTS[self.current_layout]
                    .iter()
                    .enumerate()
                    .zip(self.matrix.prev.iter())
                    .zip(self.matrix.cur.iter())
                    .filter(|(((index, _), _), _)| !self.mods.is_active(*index))
                {
                    match layout {
                        k if (k >= &KC::A && k <= &KC::Yen) => {
                            if *mat_prev == 0 && *mat_cur > 0 {
                                if self.homerow.is_empty() {
                                    k.usb_code(&self.mods, &mut key_buffer);
                                    self.last_index_printed = index;
                                } else {
                                    self.homerow.push_back(KeyIndex { key: *k, index }).ok();
                                }
                            }
                        }
                        k if (k >= &KC::ACircum && k <= &KC::YDiaer) => {
                            if *mat_prev == 0 && *mat_cur > 0 {
                                if self.homerow.is_empty() {
                                    k.usb_code(&self.mods, &mut key_buffer);
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
                                    LAYOUTS[self.current_layout]
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
                                } else {
                                    if self.mouse_scroll_tempo >= scroll_temp {
                                        self.mouse_scroll_tempo = 0;
                                        mouse_report = k.usb_mouse_move(mouse_report, scroll_speed);
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                }

                // Close repeat if needed -----------------------------------------------
                // Last key is automatically repeated by the usb crate
                if self.last_index_printed != usize::MAX
                    && self.matrix.cur[self.last_index_printed] == 0
                {
                    KC::None.usb_code(&self.mods, &mut key_buffer);
                    self.last_index_printed = usize::MAX;
                }
            }

            // --
            self.homerow_history
                .retain(|&index| !(self.matrix.prev[index] > 0 && self.matrix.cur[index] == 0));
            self.layouts.retain_mut(|l| match l {
                Lay::Pressed(_, index) => self.matrix.cur[*index] > 0,
                Lay::Dead(_, index, done) => {
                    if !*done {
                        if self.matrix.prev[*index] == 0 && self.matrix.cur[*index] > 0 {
                            *done = true;
                        } else if self.matrix.cur[*index] == 0 {
                            *done = self.matrix.cur.iter().filter(|c| **c > 0).count()
                                > self.mods.nb_on()
                        } else if self.matrix.cur[*index] > HOLD_TIME {
                            *done = self.matrix.cur.iter().filter(|c| **c > 0).count()
                                > self.mods.nb_on() + 1
                        }
                    }

                    !(*done && self.matrix.cur[*index] < HOLD_TIME)
                }
            });
        }

        // --
        self.led_status = 0;
        match self.layouts.last().unwrap_or(&Lay::Pressed(0, 0)) {
            Lay::Dead(4, _, _) => self.led_status = LED_LAYOUT_FR,
            _ => {}
        }
        if self.leader_key {
            self.led_status = LED_LEADER_KEY;
        }

        (key_buffer, mouse_report, self.led_status)
    }
}

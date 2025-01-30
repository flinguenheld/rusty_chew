use crate::{
    keys::{Lay, KC},
    layouts::LAYOUTS,
    utils::{
        led::LED_LAYOUT_FR,
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

    // Allow to drastically slow down the wheel
    mouse_scroll_tempo: u32,
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

            mouse_scroll_tempo: 0,
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
                                    // led_status = LED_LAYOUT_FR;

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

            // Modifiers ----------------------------------------------------------------
            LAYOUTS[self.current_layout]
                .iter()
                .zip(self.matrix.cur.iter())
                .enumerate()
                .filter(|(_, (&la, &mc))| {
                    mc > 0
                        && ((la >= KC::Alt && la <= KC::Shift)
                            || (la >= KC::HomeAltA && la <= KC::HomeSftR))
                })
                .for_each(|(index, (layout, _))| match layout {
                    KC::Alt => self.mods.alt = (true, index),
                    KC::Altgr => self.mods.alt_gr = (true, index),
                    KC::Ctrl => self.mods.ctrl = (true, index),
                    KC::Gui => self.mods.gui = (true, index),
                    KC::Shift => self.mods.shift = (true, index),

                    KC::HomeAltA | KC::HomeAltU => self.mods.alt = (false, index),
                    KC::HomeCtrlE | KC::HomeCtrlT => self.mods.ctrl = (false, index),
                    KC::HomeGuiS | KC::HomeGuiI => self.mods.gui = (false, index),
                    _ => self.mods.shift = (false, index),
                });

            self.mods.update_states(&self.matrix.cur);

            // Regular keys -------------------------------------------------------------
            for (((index, layout), mat_prev), mat_cur) in LAYOUTS[self.current_layout]
                .iter()
                .enumerate()
                .zip(self.matrix.prev.iter())
                .zip(self.matrix.cur.iter())
                .filter(|(((index, _), _), _)| !self.mods.is_active(*index))
            {
                match layout {
                    k if (k >= &KC::A && k <= &KC::Yen) => {
                        // Last key is automatically repeated by the usb crate
                        if *mat_prev == 0 && *mat_cur > 0 {
                            k.usb_code(&self.mods, &mut key_buffer);
                        } else if *mat_prev > 0 && *mat_cur == 0 {
                            KC::None.usb_code(&self.mods, &mut key_buffer);
                        }
                    }
                    k if (k >= &KC::ACircum && k <= &KC::YDiaer) => {
                        if *mat_prev == 0 && *mat_cur > 0 {
                            k.usb_code(&self.mods, &mut key_buffer);
                        }
                    }
                    k if (k >= &KC::HomeAltA && k <= &KC::HomeSftR) => {
                        // To validate the release, the press event has to be saved in the history
                        if *mat_prev == 0 && *mat_cur > 0 {
                            self.homerow_history.insert(index).ok();
                        } else if *mat_prev > 0
                            && *mat_prev < HOLD_TIME
                            && *mat_cur == 0
                            && self.homerow_history.contains(&index)
                        {
                            k.usb_code(&self.mods, &mut key_buffer);
                        } else if *mat_prev > 0 && *mat_cur == 0 {
                            KC::None.usb_code(&self.mods, &mut key_buffer);
                        }
                    }

                    // Mouse ------------------------------------------------------------
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
                                        **k >= KC::MouseSpeed1 && **k <= KC::MouseSpeed4 && **m > 0
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

        self.led_status = 0;
        if let Some(last) = self.layouts.last() {
            match last {
                Lay::Dead(4, _, _) => self.led_status = LED_LAYOUT_FR,
                _ => {}
            }
        }

        (key_buffer, mouse_report, self.led_status)
    }
}

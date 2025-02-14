use super::{
    matrix::Matrix,
    options::{
        MOUSE_SPEED_1, MOUSE_SPEED_2, MOUSE_SPEED_3, MOUSE_SPEED_4, MOUSE_SPEED_DEFAULT,
        SCROLL_TEMP_SPEED_1, SCROLL_TEMP_SPEED_2, SCROLL_TEMP_SPEED_3, SCROLL_TEMP_SPEED_4,
        SCROLL_TEMP_SPEED_DEFAULT,
    },
};
use crate::{chew::Key, keys::KC};

use heapless::Vec;
use usbd_human_interface_device::device::mouse::WheelMouseReport;

/// Allows Chew to emulate the mouse.
/// Speeds have to be maintained to be effective.
pub struct Mouse {
    buttons: Vec<(usize, u8), 3>,
    speed_button: (bool, usize),
    speed: i8,
    scroll_speed: (u32, i8),
    scroll_tempo: u32, // Slow down the wheel
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            buttons: Vec::new(),
            speed_button: (false, 0),
            speed: MOUSE_SPEED_DEFAULT,
            scroll_speed: SCROLL_TEMP_SPEED_DEFAULT,
            scroll_tempo: 0,
        }
    }

    pub fn speed(&mut self, key: &Key) {
        self.speed_button = (true, key.index);
        // self.scroll_tempo = 0;
        (self.speed, self.scroll_speed) = match key.code {
            KC::MouseSpeed1 => (MOUSE_SPEED_1, SCROLL_TEMP_SPEED_1),
            KC::MouseSpeed2 => (MOUSE_SPEED_2, SCROLL_TEMP_SPEED_2),
            KC::MouseSpeed3 => (MOUSE_SPEED_3, SCROLL_TEMP_SPEED_3),
            KC::MouseSpeed4 => (MOUSE_SPEED_4, SCROLL_TEMP_SPEED_4),
            _ => (MOUSE_SPEED_DEFAULT, SCROLL_TEMP_SPEED_DEFAULT),
        };
    }

    pub fn movement(&self, report: &mut WheelMouseReport, key: KC) {
        match key {
            KC::MouseLeft => report.x = i8::saturating_add(report.x, -self.speed),
            KC::MouseDown => report.y = i8::saturating_add(report.y, self.speed),
            KC::MouseUp => report.y = i8::saturating_add(report.y, -self.speed),
            KC::MouseRight => report.x = i8::saturating_add(report.x, self.speed),
            _ => {}
        }
    }
    pub fn scroll(&mut self, report: &mut WheelMouseReport, key: KC) {
        self.scroll_tempo += 1;

        if self.scroll_tempo >= self.scroll_speed.0 {
            self.scroll_tempo = 0;

            match key {
                KC::MouseWheelLeft => {
                    report.horizontal_wheel =
                        i8::saturating_add(report.horizontal_wheel, self.scroll_speed.1)
                }
                KC::MouseWheelDown => {
                    report.vertical_wheel =
                        i8::saturating_add(report.vertical_wheel, -self.scroll_speed.1)
                }
                KC::MouseWheelUp => {
                    report.vertical_wheel =
                        i8::saturating_add(report.vertical_wheel, self.scroll_speed.1)
                }
                KC::MouseWheelRight => {
                    report.horizontal_wheel =
                        i8::saturating_add(report.horizontal_wheel, -self.scroll_speed.1)
                }
                _ => {}
            }
        }
    }

    pub fn active_button(&mut self, mouse_report: &mut WheelMouseReport, key: &Key) {
        match key.code {
            KC::MouseBtLeft => {
                self.buttons.push((key.index, 0x1)).ok();
                mouse_report.buttons |= 0x1;
            }
            KC::MouseBtMiddle => {
                self.buttons.push((key.index, 0x4)).ok();
                mouse_report.buttons |= 0x4;
            }
            _ => {
                self.buttons.push((key.index, 0x2)).ok();
                mouse_report.buttons |= 0x2;
            }
        }
    }

    /// Button values have to be updated when the button is released
    pub fn release(&mut self, matrix: &Matrix, mouse_report: &mut WheelMouseReport) {
        for (index, bt_value) in self.buttons.iter_mut() {
            if !matrix.is_active(*index) {
                mouse_report.buttons &= 0xFF - *bt_value;
                *index = usize::MAX;
            }
        }

        self.buttons.retain(|(i, _)| *i < usize::MAX);

        // Move --
        if self.speed_button.0 && !matrix.is_active(self.speed_button.1) {
            self.speed_button = (false, 0);
            self.speed = MOUSE_SPEED_DEFAULT;
            self.scroll_speed = SCROLL_TEMP_SPEED_DEFAULT;
        }
    }
}

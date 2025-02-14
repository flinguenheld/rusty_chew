use super::{
    matrix::Matrix,
    options::{
        MOUSE_SPEED_1, MOUSE_SPEED_2, MOUSE_SPEED_3, MOUSE_SPEED_4, MOUSE_SPEED_DEFAULT,
        SCROLL_SPEED_1, SCROLL_SPEED_2, SCROLL_SPEED_3, SCROLL_SPEED_4, SCROLL_SPEED_DEFAULT,
    },
};
use crate::{chew::Key, keys::KC};

use heapless::Vec;
use usbd_human_interface_device::device::mouse::WheelMouseReport;

/// Allows Chew to emulate the mouse.
/// Speeds are saved by pressing order and have to be maintained to be effective.
pub struct Mouse {
    buttons: Vec<(usize, u8), 3>,
    speeds: Vec<(usize, i8, (u32, i8)), 4>,
    scroll_tempo: u32, // Slow down the wheel
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            buttons: Vec::new(),
            speeds: Vec::new(),
            scroll_tempo: 0,
        }
    }

    pub fn speed(&mut self, key: &Key) {
        self.speeds
            .push(match key.code {
                KC::MouseSpeed1 => (key.index, MOUSE_SPEED_1, SCROLL_SPEED_1),
                KC::MouseSpeed2 => (key.index, MOUSE_SPEED_2, SCROLL_SPEED_2),
                KC::MouseSpeed3 => (key.index, MOUSE_SPEED_3, SCROLL_SPEED_3),
                _ => (key.index, MOUSE_SPEED_4, SCROLL_SPEED_4),
            })
            .ok();
        self.scroll_tempo = 0;
    }

    pub fn movement(&self, report: &mut WheelMouseReport, key: KC) {
        let speed = self.speeds.last().map_or(MOUSE_SPEED_DEFAULT, |s| s.1);

        match key {
            KC::MouseLeft => report.x = i8::saturating_add(report.x, -speed),
            KC::MouseDown => report.y = i8::saturating_add(report.y, speed),
            KC::MouseUp => report.y = i8::saturating_add(report.y, -speed),
            KC::MouseRight => report.x = i8::saturating_add(report.x, speed),
            _ => {}
        }
    }

    pub fn scroll(&mut self, report: &mut WheelMouseReport, key: KC) {
        let (tempo, speed) = self.speeds.last().map_or(SCROLL_SPEED_DEFAULT, |s| s.2);
        self.scroll_tempo += 1;

        if self.scroll_tempo >= tempo {
            self.scroll_tempo = 0;

            match key {
                KC::MouseWheelLeft => {
                    report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel, speed)
                }
                KC::MouseWheelDown => {
                    report.vertical_wheel = i8::saturating_add(report.vertical_wheel, -speed)
                }
                KC::MouseWheelUp => {
                    report.vertical_wheel = i8::saturating_add(report.vertical_wheel, speed)
                }
                KC::MouseWheelRight => {
                    report.horizontal_wheel = i8::saturating_add(report.horizontal_wheel, -speed)
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
        self.buttons
            .retain(|(index, bt_value)| match matrix.is_active(*index) {
                false => {
                    mouse_report.buttons &= 0xFF - *bt_value;
                    false
                }
                _ => true,
            });
        self.speeds.retain(|s| matrix.is_active(s.0));
    }
}

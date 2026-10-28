use usb_device::device::{UsbDevice, UsbDeviceState};
use usbd_human_interface_device::{
    device::{
        keyboard::NKROBootKeyboard,
        mouse::{WheelMouse, WheelMouseReport},
    },
    UsbHidError,
};
use waveshare_rp2040_zero::hal::usb::UsbBus;

use super::keys::{BuffCase, Buffer};
use crate::{
    hardware::led::{Led, LedColor},
    options::TIMER_USB_LOOP,
};

/// Writes the reports to the computer, the same way on mono and on the split master:
///     - keyboard: one case of the queue at most every TIMER_USB_LOOP, a lone case goes out
///       at once (see docs/TIMING.md)
///     - mouse: written straight away, not through the queue
pub struct UsbReports {
    // Last keyboard case written, an identical one is skipped
    last_printed_key: BuffCase,
    // Tick from which the next keyboard case can be written (TIMER_USB_LOOP after the last one)
    next_ticks: u32,
    // Mouse buttons of the last report, a change is written even without a move
    last_mouse_buttons: u8,
}

impl UsbReports {
    pub fn new() -> Self {
        UsbReports {
            last_printed_key: BuffCase::default(),
            next_ticks: 0,
            last_mouse_buttons: 0,
        }
    }

    /// Write the first case of the queue once the gap is over.
    /// WouldBlock: the computer hasn't read the last report yet, the case waits for the next
    /// call. While the USB is suspended, it wakes the computer up and the queue is emptied.
    pub fn write_keyboard(
        &mut self,
        keyboard: &mut NKROBootKeyboard<'_, UsbBus>,
        usb_dev: &UsbDevice<'_, UsbBus>,
        key_buffer: &mut Buffer,
        led: &mut Led,
        ticks: u32,
    ) {
        if (ticks.wrapping_sub(self.next_ticks) as i32) < 0 {
            return;
        }

        if let Some(popped_key) = key_buffer.keys.pop_front() {
            if popped_key != self.last_printed_key {
                match keyboard.write_report(popped_key.key_code.clone()) {
                    Err(UsbHidError::WouldBlock) => {
                        // Wake up --
                        if usb_dev.state() == UsbDeviceState::Suspend {
                            usb_dev.bus().remote_wakeup();
                            key_buffer.keys.clear();
                        } else {
                            key_buffer.keys.push_front(popped_key).ok();
                        }
                    }
                    Err(UsbHidError::Duplicate) => {
                        led.on(LedColor::Blue);
                    }
                    Ok(_) => {
                        self.next_ticks = ticks.wrapping_add(TIMER_USB_LOOP);
                        self.last_printed_key = popped_key;
                    }
                    Err(e) => {
                        core::panic!("Failed to write keyboard report: {:?}", e)
                    }
                }
            }
        }
    }

    /// Write the mouse report when it moves or its buttons changed, then empty it.
    pub fn write_mouse(
        &mut self,
        mouse: &mut WheelMouse<'_, UsbBus>,
        report: &mut WheelMouseReport,
        led: &mut Led,
    ) {
        if report.buttons != self.last_mouse_buttons
            || report.x != 0
            || report.y != 0
            || report.vertical_wheel != 0
            || report.horizontal_wheel != 0
        {
            match mouse.write_report(report) {
                Err(UsbHidError::WouldBlock) => {
                    led.on(LedColor::Red);
                }
                Ok(_) => {
                    self.last_mouse_buttons = report.buttons;
                    *report = WheelMouseReport::default();
                }
                Err(e) => {
                    core::panic!("Failed to write mouse report: {:?}", e)
                }
            };
        }
    }
}

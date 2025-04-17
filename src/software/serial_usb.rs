use core::fmt::{Display, Write};
use heapless::String;
use usbd_serial::SerialPort;
use waveshare_rp2040_zero::hal::usb::UsbBus;

use crate::options::SERIAL_ON;

pub fn serial_write(serial: &mut SerialPort<'_, UsbBus>, txt: &str) {
    if SERIAL_ON {
        serial.write(txt.as_bytes()).ok();
    }
}

#[allow(dead_code)]
pub fn serial_write_value<T: Display>(
    serial: &mut SerialPort<'_, UsbBus>,
    before: &str,
    value: T,
    after: &str,
) {
    let mut l: String<70> = String::new();
    write!(&mut l, "{} {} {}", before, value, after).ok();

    serial_write(serial, l.as_str());
}

#[allow(dead_code)]
pub fn serial_write_values<T: Display>(
    serial: &mut SerialPort<'_, UsbBus>,
    before: &str,
    values: &[T],
    after: &str,
) {
    if !values.is_empty() {
        let mut l: String<70> = String::new();

        write!(&mut l, "{}", before).ok();
        for v in values.iter() {
            write!(&mut l, "{:2}  ", v).ok();
        }
        write!(&mut l, "{}", after).ok();

        serial_write(serial, l.as_str());
    }
}

#[allow(dead_code)]
pub fn serial_write_time(
    serial: &mut SerialPort<'_, UsbBus>,
    before: &str,
    ticks: u32,
    after: &str,
) {
    let mut l: String<70> = String::new();

    let ms = ticks / 1000;
    let seconds = ms % 60;
    let minutes = (ms / 60) % 60;
    let hours = (ms / 60) / 60;

    write!(
        &mut l,
        // "{} {:02}:{:02}:{:02}:{} {}",
        "{} {:02}:{:02}:{:02} {}",
        before,
        hours,
        minutes,
        seconds,
        // ticks % 1000,
        after
    )
    .ok();

    serial_write(serial, l.as_str());
}

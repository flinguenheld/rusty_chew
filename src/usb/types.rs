use embassy_rp::{peripherals::USB, usb};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel as ChannelSync,
};
use heapless::Vec;
use static_cell::StaticCell;

use crate::{
    options::SERIAL_MSG_MAX_LEN,
    usb::{device::UsbBuffers, serial::UsbSerial},
};

/// The RP2040 USB driver
pub type UsbDriver = usb::Driver<'static, USB>;

/// A message, sent or received (received: always one line).
pub type Message = Vec<u8, SERIAL_MSG_MAX_LEN>;

/// A queue of `N` messages, shared between tasks.
pub type MsgChannel<const N: usize> =
    ChannelSync<CriticalSectionRawMutex, Message, N>;

/// The memory of the USB device, filled once in `main`.
pub static USB_BUFFERS: StaticCell<UsbBuffers> = StaticCell::new();

/// The only serial, used from every task: `SERIAL.print(b"hello\n")`.
pub static SERIAL: UsbSerial = UsbSerial::new();

use embassy_rp::{peripherals::USB, usb};
use embassy_usb::class::cdc_acm::State;
use static_cell::StaticCell;

/// The memory `embassy-usb` borrows: it has no heap, so we give it.
/// In one `static`, so it lives as long as the USB device (`'static`).
pub struct UsbBuffers {
    pub config_descriptor: [u8; 256],
    pub bos_descriptor: [u8; 256],
    pub control_buf: [u8; 64],
    pub state: State<'static>,
}

pub static USB_BUFFERS: StaticCell<UsbBuffers> = StaticCell::new();

/// The RP2040 USB driver, `'static` so the device can go in a task.
pub type UsbDriver = usb::Driver<'static, USB>;

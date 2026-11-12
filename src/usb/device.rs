use embassy_usb::class::cdc_acm::State;

/// The memory `embassy-usb` borrows: it has no heap, so we give it.
/// In one `static`, so it lives as long as the USB device (`'static`).
pub struct UsbBuffers {
    pub config_descriptor: [u8; 256],
    pub bos_descriptor: [u8; 256],
    pub control_buf: [u8; 64],
    pub state: State<'static>,
}

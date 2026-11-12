use crate::{
    options::SERIAL_PACKET_SIZE,
    usb::types::{Message, UsbDriver, SERIAL},
};
use core::mem;
use embassy_usb::class::cdc_acm::Receiver;

/// Read the serial & fill the rx channel with lines received.
pub struct UsbSerialRx {
    rx: Receiver<'static, UsbDriver>,
    message: Message,
}

impl UsbSerialRx {
    /// Take the USB receiver & the channel to fill.
    pub fn new(rx: Receiver<'static, UsbDriver>) -> Self {
        Self { rx, message: Message::new() }
    }
}

impl UsbSerialRx {
    /// Wait for a packet & push its bytes.
    pub async fn read_packet(&mut self) {
        let mut packet = [0_u8; SERIAL_PACKET_SIZE];
        let nb = self.rx.read_packet(&mut packet).await.unwrap_or(0);

        Self::echo(&packet[..nb]);
        for &byte in &packet[..nb] {
            self.push_byte(byte);
        }
    }

    /// Add a byte to the message.
    /// Try to send the message at the end of the line.
    /// (if the channel is full, the message is lost)
    fn push_byte(&mut self, byte: u8) {
        if byte == b'\r' || byte == b'\n' {
            let message = mem::take(&mut self.message);
            if !message.is_empty() {
                SERIAL.channel_rx.try_send(message).ok();
            }
        } else {
            self.message.push(byte).ok();
        }
    }

    /// Print the letters sent by the user.
    fn echo(packet: &[u8]) {
        let msg: Message = packet
            .iter()
            .map(|&c| if c == b'\r' { b'\n' } else { c })
            .collect();
        SERIAL.print(&msg);
    }
}

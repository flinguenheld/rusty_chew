use embassy_usb::class::cdc_acm::Sender;

use crate::{
    options::SERIAL_PACKET_SIZE,
    usb::types::{Message, UsbDriver, SERIAL},
};

/// Format & send a message through the serial.
pub struct UsbSerialTx {
    tx: Sender<'static, UsbDriver>,
    message: Message,
}

impl UsbSerialTx {
    /// Take the USB sender & the channel to empty.
    pub fn new(tx: Sender<'static, UsbDriver>) -> Self {
        Self { tx, message: Message::new() }
    }
}

impl UsbSerialTx {
    /// Wait for the next message & send it.
    pub async fn send_message(&mut self) {
        let next = SERIAL.channel_tx.receive().await;
        self.format(&next).write().await;
    }

    /// Copy the source, with each '\n' as "\r\n".
    fn format(&mut self, source: &[u8]) -> &mut Self {
        self.message.clear();
        for &byte in source.iter() {
            let bytes: &[u8] = match byte {
                b'\n' => b"\r\n",
                _ => &[byte],
            };

            // All or nothing: never a lone '\r' at the end
            if self.message.extend_from_slice(bytes).is_err() {
                break;
            }
        }

        self
    }

    /// Send the message, packet by packet.
    /// A packet shorter than `SERIAL_PACKET_SIZE` ends the transfer: after
    /// a full last one, send an empty one (ZLP, Zero-Length Packet).
    async fn write(&mut self) -> &mut Self {
        self.tx.wait_connection().await;

        for packet in self.message.chunks(SERIAL_PACKET_SIZE) {
            self.tx.write_packet(packet).await.ok();
        }

        // Last packet full (or empty message): add the ZLP
        if self.message.len().rem_euclid(SERIAL_PACKET_SIZE) == 0 {
            self.tx.write_packet(&[]).await.ok();
        }

        self
    }
}

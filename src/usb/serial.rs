use embassy_sync::channel::Channel as ChannelSync;

use crate::{
    options::{SERIAL_MSG_MAX_LEN, SERIAL_RX_QUEUE_LEN, SERIAL_TX_QUEUE_LEN},
    usb::types::{Message, MsgChannel},
};

/// Manage the channels to write & read.
pub struct UsbSerial {
    pub(crate) channel_tx: MsgChannel<SERIAL_TX_QUEUE_LEN>,
    pub(crate) channel_rx: MsgChannel<SERIAL_RX_QUEUE_LEN>,
}

impl UsbSerial {
    pub const fn new() -> Self {
        Self { channel_tx: ChannelSync::new(), channel_rx: ChannelSync::new() }
    }

    /// Queue the bytes to send, without waiting.
    /// Cut if too loog and dropped if the queue is full.
    pub fn print(&self, bytes: &[u8]) {
        let message = bytes.iter().copied().take(SERIAL_MSG_MAX_LEN).collect();
        self.channel_tx.try_send(message).ok();
    }

    /// Wait for the next message received.
    pub async fn read(&self) -> Message {
        self.channel_rx.receive().await
    }
}

impl Default for UsbSerial {
    fn default() -> Self {
        Self::new()
    }
}

use embassy_usb::class::cdc_acm::Receiver;
use embassy_usb::class::cdc_acm::Sender;

use crate::usb::serial_rx::UsbSerialRx;
use crate::usb::serial_tx::UsbSerialTx;
use crate::usb::types::UsbDriver;

// Group the two basic tasks, listen & write

/// Read the lines from the PC.
#[embassy_executor::task]
pub async fn serial_rx_run(receiver: Receiver<'static, UsbDriver>) {
    let mut rx = UsbSerialRx::new(receiver);

    loop {
        rx.read_packet().await;
    }
}

/// Send each message of the TX channel to the PC.
#[embassy_executor::task]
pub async fn serial_tx_run(sender: Sender<'static, UsbDriver>) {
    let mut tx = UsbSerialTx::new(sender);

    loop {
        tx.send_message().await;
    }
}

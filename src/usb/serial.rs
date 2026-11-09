use embassy_usb::class::cdc_acm::CdcAcmClass;
use heapless::Vec;

use crate::{
    options::SERIAL_MAX_TXT_LEN, options::SERIAL_PACKET_SIZE,
    usb::device::UsbDriver,
};

/// The USB serial (CDC ACM), to debug without a probe.
///
/// Owns the CDC ACM class and a text buffer (no heap): `set` stores the
/// text, `write` sends it. The PC sees a `/dev/ttyACM*` port (`tio`).
///
/// ```text
/// serial.set(b"hello\n").write().await;
/// ```
pub struct UsbSerial {
    class: CdcAcmClass<'static, UsbDriver>,
    text: Vec<u8, SERIAL_MAX_TXT_LEN>,
}

impl UsbSerial {
    pub fn new(class: CdcAcmClass<'static, UsbDriver>) -> Self {
        Self { class, text: Vec::new() }
    }

    /// Store a new text. Nothing is sent until `write`.
    ///
    /// Convert all `\n` into `\r\n` (terminals needs both).
    /// Limited to `SERIAL_MAX_TXT_LEN` bytes, the rest is dropped.
    pub fn set(&mut self, data: &[u8]) -> &mut Self {
        self.text.clear();
        for &d in data.iter() {
            let bytes: &[u8] = match d {
                b'\n' => b"\r\n",
                _ => &[d],
            };

            // All or nothing: never a lone '\r' at the end
            if self.text.extend_from_slice(bytes).is_err() {
                break;
            }
        }

        self
    }

    /// Send the stored text, in packets of `SERIAL_PACKET_SIZE` bytes.
    ///
    /// Waits until a terminal is open. Errors (unplugged) are ignored.
    ///
    /// The host shows the data only after a short packet, so a full last
    /// packet is followed by an empty one (ZLP, zero-length packet):
    ///
    /// ```text
    /// 123 bytes  [64][59]      short last packet: done
    ///  64 bytes  [64][0]       full last packet: ZLP
    /// ```
    pub async fn write(&mut self) -> &mut Self {
        self.class.wait_connection().await;

        for packet in self.text.chunks(SERIAL_PACKET_SIZE) {
            let _ = self.class.write_packet(packet).await;
        }

        // Last packet full (or no text): close the transfer with a ZLP
        if self.text.len().rem_euclid(SERIAL_PACKET_SIZE) == 0 {
            let _ = self.class.write_packet(&[]).await;
        }

        self
    }
}

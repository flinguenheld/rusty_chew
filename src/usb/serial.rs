use core::mem;

use embassy_usb::class::cdc_acm::CdcAcmClass;
use heapless::{spsc::Queue, Vec};

use crate::{
    options::SERIAL_MAX_TXT_LEN, options::SERIAL_PACKET_SIZE,
    usb::device::UsbDriver,
};

/// The USB serial (CDC ACM), to debug without a probe.
///
/// Owns the CDC ACM class and its buffers (no heap). The PC sees a
/// `/dev/ttyACM*` port (`tio`).
///
/// - Send: `set` stores the text, `write` sends it.
/// - Receive: `read` waits for a packet and cuts it into lines,
///   `get_line` gives them back, oldest first.
///
/// ```text
/// serial.set(b"hello\n").write().await;
///
/// serial.read().await;
/// while let Some(line) = serial.get_line() { … }
/// ```
pub struct UsbSerial {
    class: CdcAcmClass<'static, UsbDriver>,
    text: Vec<u8, SERIAL_MAX_TXT_LEN>,
    rcp_buffer: Vec<u8, SERIAL_MAX_TXT_LEN>,
    rcp_lines: Queue<Vec<u8, SERIAL_MAX_TXT_LEN>, 10>,
}

// Build
impl UsbSerial {
    /// Wrap the CDC ACM class built in `main`. All buffers start empty.
    pub fn new(class: CdcAcmClass<'static, UsbDriver>) -> Self {
        Self {
            class,
            text: Vec::new(),
            rcp_buffer: Vec::new(),
            rcp_lines: Queue::new(),
        }
    }
}

// Send
impl UsbSerial {
    /// Store a new text. Nothing is sent until `write`.
    ///
    /// Each `\n` becomes `\r\n`: terminals need both to start a new line.
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
            self.class.write_packet(packet).await.ok();
        }

        // Last packet full (or no text): close the transfer with a ZLP
        if self.text.len().rem_euclid(SERIAL_PACKET_SIZE) == 0 {
            self.class.write_packet(&[]).await.ok();
        }

        self
    }
}

// Receive
impl UsbSerial {
    /// Wait for one packet from the PC and add its bytes to the lines.
    ///
    /// The task sleeps until a packet arrives (woken by USBCTRL_IRQ), so
    /// call it in a loop. A terminal sends each key at once: packets are
    /// often 1 byte.
    ///
    /// On the RP2040, `read_packet` only waits: its one error is a buffer
    /// too small, avoided by `SERIAL_PACKET_SIZE`. So `unwrap_or(0)` can't
    /// turn the loop into a busy loop.
    pub async fn read(&mut self) -> &mut Self {
        let mut buffer = [0_u8; SERIAL_PACKET_SIZE];
        let nb = self.class.read_packet(&mut buffer).await.unwrap_or(0);

        for &b in &buffer[..nb] {
            self.push_byte(b);
        }

        self
    }

    /// Add one byte to the current line, or end it on `\r` or `\n`.
    ///
    /// Empty lines are skipped, so every terminal gives the same lines:
    ///
    /// ```text
    /// "ab\r"    → "ab"           (tio)
    /// "ab\n"    → "ab"           (printf, line mode)
    /// "ab\r\n"  → "ab", "" skipped
    /// ```
    ///
    /// A line longer than `SERIAL_MAX_TXT_LEN` is cut. When the queue is
    /// full, new lines are dropped.
    fn push_byte(&mut self, byte: u8) {
        if byte == b'\r' || byte == b'\n' {
            let line = mem::take(&mut self.rcp_buffer);
            if !line.is_empty() {
                self.rcp_lines.enqueue(line).ok();
            }
        } else {
            self.rcp_buffer.push(byte).ok();
        }
    }

    /// Take the oldest complete line, or `None` if there is none.
    pub fn get_line(&mut self) -> Option<Vec<u8, SERIAL_MAX_TXT_LEN>> {
        self.rcp_lines.dequeue()
    }
}

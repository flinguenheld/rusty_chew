use heapless::{String, Vec};
use waveshare_rp2040_zero as bsp;

use bsp::{
    hal::{
        gpio::bank0::Gpio11,
        gpio::{FunctionPio0, Pin, PullUp},
        pio::{UninitStateMachine, PIO, SM1, SM2},
    },
    pac::PIO0,
};
use cortex_m::delay::Delay;
use fugit::HertzU32;

use crate::hardware::half_duplex::HalfDuplex;
use crate::options::{MAX_MESSAGE_LENGTH, MAX_NOT_COMPLETE, UART_SEND_DELAY, UART_SPEED};

// Header = who sends the frame. It marks the start of a frame, and a side reading its own
// frame back (echo) rejects it. Both differ by all their bits, and are far from 0x00 / 0xFF.
pub const HEADER_MASTER: u8 = 0b10100101;
pub const HEADER_SLAVE: u8 = 0b01011010;

// Every frame has the same length: header + values + CRC
pub const FRAME_VALUES: usize = 3;
const FRAME_LEN: usize = FRAME_VALUES + 2;

pub enum UartError {
    Crc,
    Header,
    NothingToRead,
    NotComplete,
}

#[allow(dead_code)]
impl UartError {
    pub fn to_serial(&self) -> String<40> {
        let mut output = String::new();
        output
            .push_str(match self {
                UartError::Crc => "-- Error CRC --\r\n",
                UartError::Header => "-- Error Header --\r\n",
                UartError::NothingToRead => "-- Error Nothing to read --\r\n",
                UartError::NotComplete => "-- Error Not complete --\r\n",
            })
            .ok();
        output
    }
}

/// Message Frame, always 5 bytes
///
///  Header            Values (3 bytes)                  CRC-8
///    /                    /                              /
/// 1010 0101  ->  0000 0000 - 0000 0000 - 0000 0000  ->  0000 0000
///
///     HEADER_MASTER   Status bits (LED + buzzer state), 0, 0 (free)
///     HEADER_SLAVE    Mask of the slave's pressed keys (17 bits)
///
/// The CRC is computed over the header and the values.
/// Values can take any value (keys mask), so a header is only searched at the buffer start.
pub struct Mail {
    pub header: u8,
    pub values: [u8; FRAME_VALUES],
}

/// Half-duplex uart (see half_duplex.rs), the line is released except to send a mail.
/// The RP2040-zero board buffer is limited to 9 bytes, so a frame has a fixed length of 5 bytes:
/// one byte as a header to know who sends it, 3 values, and one CRC-8 byte at the end to
/// reject corrupted frames.
/// The purpose for Chew is (Both sides read their buffer each n ms):
///     - Master sends its statuses each n milliseconds, as a request
///     - Slave updates its LED & buzzer, and answers with its pressed keys mask
///     - Master applies the chew logic when the slave message is validated
/// Only the master starts an exchange, so both sides never send at the same time.
///
/// In case of error, the buffer is emptied (the chew loop is lost).
/// After a valid frame, the following bytes are kept for the next mail.
pub struct Uart {
    line: HalfDuplex,

    // 9 bytes is the buffer's maximum
    buffer: Vec<u8, MAX_MESSAGE_LENGTH>,

    counter_not_complete: u32,
}

impl Uart {
    pub fn new(
        pio: &mut PIO<PIO0>,
        sm1: UninitStateMachine<(PIO0, SM1)>,
        sm2: UninitStateMachine<(PIO0, SM2)>,
        gp11: Pin<Gpio11, FunctionPio0, PullUp>,
        sys_freq: HertzU32,
    ) -> Uart {
        Uart {
            line: HalfDuplex::new(pio, sm1, sm2, gp11, UART_SPEED, sys_freq),
            buffer: Vec::new(),
            counter_not_complete: 0,
        }
    }

    /// Send a mail, return once it is on the wire
    pub fn send(&mut self, header: u8, values: [u8; FRAME_VALUES], delay: &mut Delay) {
        // Bytes already received are kept, the line only drops the echo of what is sent
        self.read_uart_buffer();

        // Short guard, the other side may still be ending its stop bit
        delay.delay_us(UART_SEND_DELAY);

        let mut frame = [header, values[0], values[1], values[2], 0];
        frame[FRAME_LEN - 1] = crc8(&frame[..FRAME_LEN - 1]);
        self.line.write(&frame, delay);
    }

    /// Empty the uart buffer to fill the struct buffer.
    /// Proceede to checks and return a mail which contains the header & the values.
    pub fn receive(&mut self) -> Result<Mail, UartError> {
        self.read_uart_buffer();

        if let Some(&header) = self.buffer.first() {
            if header != HEADER_MASTER && header != HEADER_SLAVE {
                self.buffer.clear();
                self.counter_not_complete = 0;
                Err(UartError::Header)
            } else if self.buffer.len() >= FRAME_LEN {
                self.counter_not_complete = 0;

                // The CRC of a frame which includes its own CRC is 0
                if crc8(&self.buffer[..FRAME_LEN]) == 0 {
                    let mut values = [0; FRAME_VALUES];
                    values.copy_from_slice(&self.buffer[1..=FRAME_VALUES]);

                    // Keep the start of the next frame
                    self.buffer = self.buffer.iter().skip(FRAME_LEN).cloned().collect();
                    Ok(Mail { header, values })
                } else {
                    // Frame boundaries can't be trusted anymore
                    self.buffer.clear();
                    Err(UartError::Crc)
                }
            } else if self.counter_not_complete > MAX_NOT_COMPLETE {
                self.counter_not_complete = 0;
                self.buffer.clear();
                Err(UartError::NothingToRead)
            } else {
                self.counter_not_complete += 1;
                Err(UartError::NotComplete)
            }
        } else {
            Err(UartError::NothingToRead)
        }
    }

    fn read_uart_buffer(&mut self) {
        while let Some(v) = self.line.read() {
            self.buffer.push(v).ok();
        }
    }
}

/// CRC-8 (poly 0x07, init 0), bitwise to avoid a lookup table.
/// Frames are a few bytes long every 5 ms, so speed doesn't matter here.
fn crc8(bytes: &[u8]) -> u8 {
    // CRC-8 generator x^8 + x^2 + x + 1 (top bit implied)
    let crc8_poly: u8 = 0x07;

    let mut crc = 0u8;
    for b in bytes {
        crc ^= b;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ crc8_poly;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

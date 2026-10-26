use heapless::{String, Vec};
use waveshare_rp2040_zero as bsp;

use bsp::{
    hal::{
        gpio::bank0::Gpio11,
        gpio::{FunctionPio0, Pin, PullUp},
        pio::{Running, SM1},
        pio::{UninitStateMachine, PIO},
    },
    pac::PIO0,
};
use cortex_m::delay::Delay;
use fugit::{HertzU32, RateExtU32};
use pio_uart::{PioUartRx, PioUartTx, RxProgram, TxProgram};
use usbd_human_interface_device::interface::ReportBuffer;

use crate::options::{MAX_MESSAGE_LENGTH, MAX_NOT_COMPLETE, UART_SEND_DELAY, UART_SPEED};

// Values used as a header to validated a message
pub const HR_KEYS: u8 = 0b11010000;
pub const HR_STATUS: u8 = 0b10110000;

// Every frame has the same length: header + values + CRC
pub const FRAME_VALUES: usize = 3;
const FRAME_LEN: usize = FRAME_VALUES + 2;

pub enum UartError {
    Crc,
    Header,
    NothingToRead,
    NotReciever,
    NotTransmitter,
    NotComplete,
    Uart,
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
                UartError::NotReciever => "-- Error Not reciever --\r\n",
                UartError::NotTransmitter => "-- Error Not transmitter --\r\n",
                UartError::NotComplete => "-- Error Not complete --\r\n",
                UartError::Uart => "-- Error Uart --\r\n",
            })
            .ok();
        output
    }
}

/// Message Frame, always 5 bytes
///
///  Header            Values (3 bytes)                  CRC-8
///    /                    /                              /
/// 1101 0000  ->  0000 0000 - 0000 0000 - 0000 0000  ->  0000 0000
///
///     HR_KEYS     Mask of the slave's pressed keys (17 bits)
///     HR_STATUS   Status bits (LED + buzzer state), 0, 0 (free)
///
/// The CRC is computed over the header and the values.
/// Values can take any value (keys mask), so a header is only searched at the buffer start.
pub struct Mail {
    pub header: u8,
    pub values: [u8; FRAME_VALUES],
}

/// Half-duplex uart, the system is on receiver mode all the time except to send a mail.
/// The RP2040-zero board buffer is limited to 9 bytes, so a frame has a fixed length of 5 bytes:
/// one byte as a header to know what they want, 3 values, and one CRC-8 byte at the end to
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
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,

    // 9 bytes is the buffer's maximum
    buffer: Vec<u8, MAX_MESSAGE_LENGTH>,

    counter_not_complete: u32,

    // System clock, the PIO clock divider is computed from it
    sys_freq: HertzU32,
}

impl Uart {
    pub fn new(
        pio: &mut PIO<PIO0>,
        sm1: UninitStateMachine<(PIO0, SM1)>,
        gp11: Pin<Gpio11, FunctionPio0, PullUp>,
        sys_freq: HertzU32,
    ) -> Uart {
        let mut uart = Uart {
            tx_program: pio_uart::install_tx_program(pio).ok().unwrap(),
            rx_program: pio_uart::install_rx_program(pio).ok().unwrap(),

            transmiter: None,
            receiver: None,

            buffer: Vec::new(),

            counter_not_complete: 0,

            sys_freq,
        };

        uart.receiver = Some(
            pio_uart::PioUartRx::new(
                gp11.reconfigure(),
                sm1,
                &mut uart.rx_program,
                UART_SPEED.Hz(),
                uart.sys_freq,
            )
            .enable(),
        );

        uart
    }

    /// Switch in transmitter mode, send, and come back to receiver mode
    pub fn send(
        &mut self,
        header: u8,
        values: [u8; FRAME_VALUES],
        delay: &mut Delay,
    ) -> Result<(), UartError> {
        let ok;
        self.switch_to_transmitter();

        delay.delay_us(UART_SEND_DELAY);
        if let Some(transmiter) = &mut self.transmiter {
            let mut frame = [header, values[0], values[1], values[2], 0];
            frame[FRAME_LEN - 1] = crc8(&frame[..FRAME_LEN - 1]);

            ok = transmiter.write_raw(&frame).is_ok();

            // write_raw returns once the bytes are queued, wait until they are on the wire
            // (10 bits per byte: start + 8 data + stop)
            delay.delay_us(FRAME_LEN as u32 * 10 * 1_000_000 / UART_SPEED);
        } else {
            return Err(UartError::NotTransmitter);
        }
        delay.delay_us(UART_SEND_DELAY);
        self.switch_to_receiver();

        match ok {
            true => Ok(()),
            false => Err(UartError::Uart),
        }
    }

    /// Empty the uart buffer to fill the struct buffer.
    /// Proceede to checks and return a mail which contains the header & the values.
    pub fn receive(&mut self) -> Result<Mail, UartError> {
        if self.read_uart_buffer().is_err() {
            Err(UartError::Uart)
        } else if let Some(&header) = self.buffer.first() {
            if header != HR_KEYS && header != HR_STATUS {
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

    fn read_uart_buffer(&mut self) -> Result<(), UartError> {
        if let Some(receiver) = &mut self.receiver {
            let mut temp_buffer = [0; MAX_MESSAGE_LENGTH];

            match receiver.read_raw(&mut temp_buffer) {
                Ok(n) => {
                    for v in temp_buffer.iter().take(n) {
                        self.buffer.push(*v).ok();
                    }
                    Ok(())
                }
                Err(_) => Err(UartError::Uart),
            }
        } else {
            Err(UartError::NotReciever)
        }
    }

    fn switch_to_transmitter(&mut self) {
        if let Some(receiver) = self.receiver.take() {
            let (sm1, gp11) = receiver.stop().free();

            self.transmiter = Some(
                pio_uart::PioUartTx::new(
                    gp11.reconfigure(),
                    sm1,
                    &mut self.tx_program,
                    UART_SPEED.Hz(),
                    self.sys_freq,
                )
                .enable(),
            );
        }
    }

    fn switch_to_receiver(&mut self) {
        if let Some(transmiter) = self.transmiter.take() {
            let (sm1, gp11) = transmiter.stop().free();

            self.receiver = Some(
                pio_uart::PioUartRx::new(
                    gp11.reconfigure(),
                    sm1,
                    &mut self.rx_program,
                    UART_SPEED.Hz(),
                    self.sys_freq,
                )
                .enable(),
            );
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

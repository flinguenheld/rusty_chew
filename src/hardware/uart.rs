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

// Values per mail type, every frame of a type has the same length
pub const KEYS_VALUES: usize = 3; // Mask of the slave's pressed keys (17 bits)
pub const STATUS_VALUES: usize = 2; // LED state + event

fn values_len(header: u8) -> Option<usize> {
    match header {
        HR_KEYS => Some(KEYS_VALUES),
        HR_STATUS => Some(STATUS_VALUES),
        _ => None,
    }
}

pub enum UartError {
    Capacity,
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
                UartError::Capacity => "-- Error Capacity --\r\n",
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

/// Message Frame
///
///  Openning                 Values (fixed number per type)     CRC-8
///    /                                                         /
/// 1010 1010   ->    0000 0000 - 0000 0000 ...   ->   0000 0000
///        \
///      Number of values (must match the type: KEYS_VALUES or STATUS_VALUES)
///
/// The CRC is computed over the header and the values.
/// Values can take any value (keys mask), so a header is only searched at the buffer start.
pub struct Mail {
    pub header: u8,
    pub values: Vec<u8, 8>,
}

/// Half-duplex uart, the system is on receiver mode all the time except to send a mail.
/// The RP2040-zero board buffer is limited to 9 bytes messages.
/// So here it uses one byte as a header to know what they want and give the amount of bytes
/// which are following, and one CRC-8 byte at the end to reject corrupted frames.
/// The purpose for Chew is (Both sides read their buffer each n ms):
///     - Slave sends its pressed keys mask each n milliseconds
///     - Master applies the chew logic when the slave message is validated
///     - Master sends the LED states to the slave
///     - Slave updates its LED
///
/// In case of error, the buffer is emptied (the chew loop is lost).
/// After a valid frame, the following bytes are kept for the next mail.
pub struct Uart {
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,

    // 9 bytes is the buffer's maximum
    buffer: Vec<u8, 9>,

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
    pub fn send(&mut self, header: u8, values: &[u8], delay: &mut Delay) -> Result<(), UartError> {
        let ok;
        self.switch_to_transmitter();

        delay.delay_us(UART_SEND_DELAY);
        if let Some(transmiter) = &mut self.transmiter {
            if values_len(header) == Some(values.len()) {
                let mut to_send: Vec<u8, MAX_MESSAGE_LENGTH> = Vec::new();
                to_send.push(header | (values.len() as u8)).ok();
                values.iter().for_each(|v| to_send.push(*v).unwrap());
                to_send.push(crc8(&to_send)).ok();

                ok = transmiter.write_raw(&to_send).is_ok();

                // write_raw returns once the bytes are queued, wait until they are on the wire
                // (10 bits per byte: start + 8 data + stop)
                delay.delay_us(to_send.len() as u32 * 10 * 1_000_000 / UART_SPEED);
            } else {
                return Err(UartError::Capacity);
            }
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
        } else if let Some(&first) = self.buffer.first() {
            let header = first & 0b11110000;
            let l = (first & 0b00001111) as usize;

            if values_len(header) == Some(l) {
                // Header + values + CRC
                let frame_len = l + 2;

                if self.buffer.len() >= frame_len {
                    // The CRC of a frame which includes its own CRC is 0
                    let m = if crc8(&self.buffer[..frame_len]) == 0 {
                        let m = Mail {
                            header,
                            values: self.buffer[1..=l].iter().cloned().collect(),
                        };
                        // Keep the start of the next frame
                        self.buffer = self.buffer.iter().skip(frame_len).cloned().collect();
                        Ok(m)
                    } else {
                        // Frame boundaries can't be trusted anymore
                        self.buffer.clear();
                        Err(UartError::Crc)
                    };
                    self.counter_not_complete = 0;

                    m
                } else if self.counter_not_complete > MAX_NOT_COMPLETE {
                    self.counter_not_complete = 0;
                    self.buffer.clear();
                    Err(UartError::NothingToRead)
                } else {
                    self.counter_not_complete += 1;
                    Err(UartError::NotComplete)
                }
            } else {
                self.buffer.clear();
                self.counter_not_complete = 0;
                Err(UartError::Header)
            }
        } else {
            Err(UartError::NothingToRead)
        }
    }

    fn read_uart_buffer(&mut self) -> Result<(), UartError> {
        if let Some(receiver) = &mut self.receiver {
            let mut temp_buffer = [0; 9];

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

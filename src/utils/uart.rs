use super::options::{UART_SEND_DELAY, UART_SPEED};

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
use fugit::RateExtU32;
use pio_uart::{PioUartRx, PioUartTx, RxProgram, TxProgram};
use usbd_human_interface_device::interface::ReportBuffer;

const MAX_MESSAGE_LENGTH: usize = 9; // Max tested in January 2025
const MAX_NOT_COMPLETE: u32 = 3;
const MAX_NOTHING_RECEIVED: u32 = 100;

// Values used as a header to validated a message
pub const HR_KEYS: u8 = 0b11010000;
pub const HR_LED: u8 = 0b10110000;

pub enum UartError {
    Capacity,
    Header,
    NothingToRead,
    NothingToReadMax,
    NotReciever,
    NotTransmitter,
    NotComplete,
    Uart,
}

impl UartError {
    pub fn to_serial(&self) -> String<40> {
        let mut output = String::new();
        output
            .push_str(match self {
                UartError::Capacity => "-- Error Capacity --\r\n",
                UartError::Header => "-- Error Header --\r\n",
                UartError::NothingToRead => "-- Error Nothing to read --\r\n",
                UartError::NothingToReadMax => "-- Error Nothing to read MAX --\r\n",
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
///  Openning                 Values (8 bytes maximum)
///    /
/// 1010 1010   ->    0000 0000 - 0000 0000 ...
///        \
///      Number of values
///    
pub struct Mail {
    pub header: u8,
    pub values: Vec<u8, 8>,
}

/// Half-duplex uart, the system is on receiver mode all the time except to send a mail.
/// The RP2040-zero board buffer is limited to 9 bytes messages.
/// So here it uses one byte as a header to know what they want and give the amount of bytes
/// which are following.
/// The purpose for Chew is:
///     - Left sends a request of the right active keys.
///     - Right receives the request and return the indexes (see gpios).
///     - Left receives them and procedes to the keyboard logic.
///     - Left sends a request of the right active keys...
///
/// In case of bug, the const MAX_NOT_COMPLETE & MAX_NOTHING_RECEIVED allow the uart struct to not get
/// stuct and return an error. In this case the left restarts the loop.
pub struct Uart {
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,

    // 9 bytes is the buffer's maximum
    buffer: Vec<u8, 9>,

    counter_not_complete: u32,
    counter_nothing_to_read: u32,
}

impl Uart {
    pub fn new(
        pio: &mut PIO<PIO0>,
        sm1: UninitStateMachine<(PIO0, SM1)>,
        gp11: Pin<Gpio11, FunctionPio0, PullUp>,
    ) -> Uart {
        let mut uart = Uart {
            tx_program: pio_uart::install_tx_program(pio).ok().unwrap(),
            rx_program: pio_uart::install_rx_program(pio).ok().unwrap(),

            transmiter: None,
            receiver: None,

            buffer: Vec::new(),

            counter_not_complete: 0,
            counter_nothing_to_read: 0,
        };

        uart.receiver = Some(
            pio_uart::PioUartRx::new(
                gp11.reconfigure(),
                sm1,
                &mut uart.rx_program,
                UART_SPEED.Hz(),
                bsp::XOSC_CRYSTAL_FREQ.Hz(),
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
            if values.len() < MAX_MESSAGE_LENGTH {
                let mut to_send: Vec<u8, MAX_MESSAGE_LENGTH> = Vec::new();
                to_send.push(header | (values.len() as u8)).ok();
                values.iter().for_each(|v| to_send.push(*v).unwrap());

                ok = transmiter.write_raw(&to_send).is_ok();
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
        } else if let Some(first) = self.buffer.first() {
            self.counter_nothing_to_read = 0;

            if first & 0b11110000 == HR_KEYS || first & 0b11110000 == HR_LED {
                let l = first & 0b00001111;

                if self.buffer.len() == (l + 1) as usize {
                    let m = Mail {
                        header: self.buffer.first().cloned().unwrap() & 0b11110000,
                        values: self.buffer.iter().skip(1).cloned().collect(),
                    };
                    self.buffer.clear();
                    self.counter_not_complete = 0;

                    Ok(m)
                } else if self.counter_not_complete > MAX_NOT_COMPLETE {
                    self.counter_not_complete = 0;
                    self.buffer.clear();
                    Err(UartError::NothingToRead)
                } else {
                    self.counter_not_complete += 1;
                    Err(UartError::NotComplete)
                }
            } else {
                Err(UartError::Header)
            }
        } else if self.counter_nothing_to_read >= MAX_NOTHING_RECEIVED {
            self.counter_nothing_to_read = 0;
            Err(UartError::NothingToReadMax)
        } else {
            self.counter_nothing_to_read += 1;
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
                    bsp::XOSC_CRYSTAL_FREQ.Hz(),
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
                    bsp::XOSC_CRYSTAL_FREQ.Hz(),
                )
                .enable(),
            );
        }
    }
}

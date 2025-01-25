use super::options::UART_SPEED;

use heapless::Vec;
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

const MAX_MESSAGE_LENGTH: usize = 9;
pub const HR_KEYS: u8 = 0b11010000;
pub const HR_PLEASE_RESTART: u8 = 0b11100111;

pub enum UartError {
    Capacity,
    Header,
    NothingToRead,
    NotReciever,
    NotTransmitter,
    NotComplete,
    Uart,
}
pub struct Mail {
    pub header: u8,
    pub values: Vec<u8, 8>,
}

pub struct Uart {
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,

    // 9 bytes is the buffer's maximum
    buffer: Vec<u8, 9>,
}

// Message Frame
//
//  Openning                 Values (8 bytes maximum)
//    /
// 1010 1010   ->    0000 0000 - 0000 0000 ...
//        \
//      Number of values
//
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

    pub fn send(&mut self, header: u8, values: &[u8], delay: &mut Delay) -> Result<(), UartError> {
        let ok;
        self.switch_to_transmitter();

        // TODO delay length ??
        delay.delay_us(500);
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
        delay.delay_us(500);
        self.switch_to_receiver();

        match ok {
            true => Ok(()),
            false => Err(UartError::Uart),
        }
    }

    pub fn receive(&mut self) -> Result<Mail, UartError> {
        if self.read_uart_buffer().is_err() {
            return Err(UartError::Uart);
        }

        if self.buffer.is_empty() {
            Err(UartError::NothingToRead)
        } else if let Some(first) = self.buffer.first() {
            if first & 0b11110000 == HR_KEYS {
                let l = first & 0b00001111;

                if self.buffer.len() == (l + 1) as usize {
                    let m = Mail {
                        header: self.buffer.first().cloned().unwrap() & 0b11110000,
                        values: self.buffer.iter().skip(1).cloned().collect(),
                    };
                    self.buffer.clear();
                    Ok(m)
                } else {
                    // TODO: Add a check to avoid infinite NotComplete ?
                    Err(UartError::NotComplete)
                }
            } else {
                Err(UartError::Header)
            }
        } else {
            Err(UartError::NotComplete)
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

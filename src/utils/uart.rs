use super::options::UART_SPEED;
use cortex_m::delay::Delay;
use embedded_io::{Read, Write};
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
use fugit::RateExtU32;
use pio_uart::{PioUartRx, PioUartTx, RxProgram, TxProgram};

// Message Frame
//
//  Opening       Title        Value       Closure
// 1010 1010 -> 0000 0000 -> 0000 0000 -> 0101 0101

const OPENING: u8 = 0b10101010;
const CLOSURE: u8 = 0b01010101;

pub enum UartError {
    Header,
    NothingToRead,
    Uart,
    NotReciever,
    NotComplete,
}

pub struct Uart {
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,

    buffer: Vec<u8, 4>,
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

    pub fn send(&mut self, value: [u8; 2], mut delay: Delay) -> Delay {
        self.switch_to_transmiter();

        if let Some(transmiter) = &mut self.transmiter {
            transmiter
                .write_all(&[OPENING, value[0], value[1], CLOSURE])
                .ok();
        }
        delay.delay_us(100);
        self.switch_to_receiver();

        delay
    }

    pub fn receive(&mut self) -> Result<[u8; 2], UartError> {
        if self.read_uart_buffer().is_err() {
            return Err(UartError::Uart);
        }

        if self.buffer.is_empty() {
            Err(UartError::NothingToRead)
        } else if !self.buffer.is_full() {
            Err(UartError::NotComplete)
        } else {
            if self.buffer[0] != OPENING || self.buffer[3] != CLOSURE {
                Err(UartError::Header)
            } else {
                let v = [self.buffer[1], self.buffer[2]];
                self.buffer.clear();
                Ok(v)
            }
        }
    }

    fn read_uart_buffer(&mut self) -> Result<(), UartError> {
        if let Some(receiver) = &mut self.receiver {
            let mut temp_buffer = [0; 32];

            return match receiver.read(&mut temp_buffer) {
                Ok(nb) => {
                    for i in 0..nb {
                        self.buffer.push(temp_buffer[i]).ok();
                    }
                    Ok(())
                }
                Err(_) => Err(UartError::Uart),
            };
        } else {
            Err(UartError::NotReciever)
        }
    }

    fn switch_to_transmiter(&mut self) {
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

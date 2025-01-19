use super::options::UART_SPEED;
use embedded_io::{Read, Write};
use waveshare_rp2040_zero::{
    self as bsp,
    hal::{
        gpio::{FunctionPio0, Pin, PullUp},
        pio::{self, UninitStateMachine, PIO},
        uart,
    },
};

use bsp::{
    hal::{
        gpio::bank0::Gpio11,
        pio::{Running, SM1},
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

pub enum ReadUartError {
    Header,
    Type,
    Value,
    NothingToRead,
    Uart,
}

#[repr(u8)]
enum UartType {
    Ask = 0b11110000,
}

pub struct Uart {
    tx_program: TxProgram<PIO0>,
    rx_program: RxProgram<PIO0>,

    transmiter: Option<PioUartTx<Gpio11, PIO0, SM1, Running>>,
    receiver: Option<PioUartRx<Gpio11, PIO0, SM1, Running>>,
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
        };

        uart.transmiter = Some(
            pio_uart::PioUartTx::new(
                gp11.reconfigure(),
                sm1,
                &mut uart.tx_program,
                UART_SPEED.Hz(),
                bsp::XOSC_CRYSTAL_FREQ.Hz(),
            )
            .enable(),
        );

        uart
    }

    pub fn send(&mut self, message_type: UartType, value: u8) -> bool {
        self.switch_to_transmiter();

        if let Some(transmiter) = &mut self.transmiter {
            return transmiter
                .write_all(&[OPENING, message_type as u8, value, CLOSURE])
                .is_ok();
        }
        false
    }

    pub fn receive(&mut self, message_type: UartType) -> Result<u8, ReadUartError> {
        self.switch_to_receiver();

        if let Some(receiver) = &mut self.receiver {
            let mut buffer = [0; 4];

            return match receiver.read_exact(&mut buffer) {
                Ok(_) => {
                    if buffer[0] != OPENING || buffer[3] != CLOSURE {
                        Err(ReadUartError::Header)
                    } else if buffer[1] != message_type as u8 {
                        Err(ReadUartError::Type)
                    } else {
                        Ok(buffer[2])
                    }
                }
                _ => Err(ReadUartError::NothingToRead),
            };
        }

        Err(ReadUartError::Uart)
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

    // fn switch_to_transmiter(
    //     &mut self,
    //     uart: PioUartRx<Gpio11, PIO0, SM1, Running>,
    //     tx_program: &mut TxProgram<PIO0>,
    // ) -> PioUartTx<Gpio11, PIO0, SM1, Running> {
    //     let (sm1, gp11) = uart.stop().free();

    //     let uart = pio_uart::PioUartTx::new(
    //         gp11.reconfigure(),
    //         sm1,
    //         tx_program,
    //         UART_SPEED.Hz(),
    //         bsp::XOSC_CRYSTAL_FREQ.Hz(),
    //     )
    //     .enable();

    //     uart
    // }

    // fn switch_to_receiver(
    //     &mut self,
    //     uart: PioUartTx<Gpio11, PIO0, SM1, Running>,
    //     rx_program: &mut RxProgram<PIO0>,
    // ) -> PioUartRx<Gpio11, PIO0, SM1, Running> {
    //     let (sm1, gp11) = uart.stop().free();

    //     let uart = pio_uart::PioUartRx::new(
    //         gp11.reconfigure(),
    //         sm1,
    //         rx_program,
    //         UART_SPEED.Hz(),
    //         bsp::XOSC_CRYSTAL_FREQ.Hz(),
    //     )
    //     .enable();

    //     uart
    // }
}

#![no_std]
#![no_main]

mod utils;
use utils::gpios::Gpios;
use utils::led::{Led, LedColor, LED_LAYOUT_FR, LED_LEADER_KEY};
use utils::options::TIMER_UART_LOOP;
use utils::uart::{Uart, UartError, HR_KEYS, HR_LED};

use waveshare_rp2040_zero as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry, pac,
    pio::PIOExt,
    timer::Timer,
    watchdog::Watchdog,
    Sio,
};
use cortex_m::prelude::*;
use defmt::*;
use defmt_rtt as _;
use fugit::ExtU32;
use panic_probe as _;
use ws2812_pio::Ws2812;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // GPIO -----
    let mut gpios: Gpios = Gpios {
        pins: [
            [
                Some(pins.gp0.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp1.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp2.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp3.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp4.into_pull_up_input().into_dyn_pin()),
            ],
            [
                Some(pins.gp29.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp28.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp27.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp26.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp15.into_pull_up_input().into_dyn_pin()),
            ],
            [
                Some(pins.gp8.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp9.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp13.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp14.into_pull_up_input().into_dyn_pin()),
                None,
            ],
            [
                Some(pins.gp5.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp6.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp7.into_pull_up_input().into_dyn_pin()),
                None,
                None,
            ],
        ],
    };

    // Led ------
    let mut neopixel = Ws2812::new(
        // The onboard NeoPixel is attached to GPIO pin #16 on the Waveshare RP2040-Zero.
        pins.neopixel.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );
    let mut led = Led::new(&mut neopixel);

    // UART -----
    let mut uart = Uart::new(&mut pio, sm1, pins.gp11.reconfigure());

    let mut uart_count_down = timer.count_down();
    uart_count_down.start(TIMER_UART_LOOP.millis());

    loop {
        if uart_count_down.wait().is_ok() {
            match uart.receive() {
                Ok(mail) => match mail.header {
                    HR_KEYS => {
                        let active_indexes = gpios.get_right_indexes();
                        if uart.send(HR_KEYS, &active_indexes, &mut delay).is_err() {
                            led.light_on(LedColor::Red);
                        }
                    }

                    HR_LED => {
                        match mail.values[0] {
                            LED_LAYOUT_FR => led.light_on(LedColor::Aqua),
                            LED_LEADER_KEY => led.light_on(LedColor::Blue),
                            _ => led.light_off(),
                        }

                        if uart.send(HR_LED, &[mail.values[0]], &mut delay).is_err() {
                            led.light_on(LedColor::Red);
                        }
                    }

                    _ => {}
                },

                Err(UartError::NothingToRead) => {
                    // led.light_on(LedColor::Blue);
                }
                Err(UartError::NothingToReadMax) => {
                    led.light_on(LedColor::Yellow);
                }
                Err(UartError::NotComplete) => {
                    // led.light_on(LedColor::Yellow);
                }
                Err(UartError::Header) => {
                    // led.light_on(LedColor::Olive);
                }
                _ => {
                    led.light_on(LedColor::Red);
                }
            }
        }

        // led.startup(TIMER_MAIN_LOOP);
        // }
    }
}

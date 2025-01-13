#![no_std]
#![no_main]

mod utils;
use crate::utils::options::TIMER_MAIN_LOOP;
use utils::gpios::Gpios;
use utils::led::{Led, LedColor};
use utils::options::UART_SPEED;

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
use embedded_io::Write;
use fugit::{ExtU32, RateExtU32};
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

    let sio = Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    info!("Starting");

    // Configure the addressable LED
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

    // UART -----
    let mut tx_program = pio_uart::install_tx_program(&mut pio).ok().unwrap();
    let mut tx = pio_uart::PioUartTx::new(
        pins.gp11.reconfigure(),
        sm1,
        &mut tx_program,
        UART_SPEED.Hz(),
        bsp::XOSC_CRYSTAL_FREQ.Hz(),
    )
    .enable();

    let mut main_count_down = timer.count_down();
    main_count_down.start(TIMER_MAIN_LOOP.millis());

    let mut led = Led::new(&mut neopixel);

    loop {
        if main_count_down.wait().is_ok() {
            let right_pins = gpios.update_states();
            if tx.write_all(&right_pins).is_err() {
                led.light_on(LedColor::Red);
                continue;
            }

            led.startup(TIMER_MAIN_LOOP);
        }
    }
}

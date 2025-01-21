#![no_std]
#![no_main]

mod utils;

use usbd_human_interface_device::interface::ReportBuffer;
use utils::options::{DELAY, TIMER_RIGHT_LOOP};
use utils::uart::Uart;

use waveshare_rp2040_zero as bsp;

use utils::gpios::Gpios;
use utils::led::{Led, LedColor};

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

    // Remove ------------------------------------------------------------------------------
    // Remove ------------------------------------------------------------------------------
    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // --
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
    let mut uart = Uart::new(&mut pio, sm1, pins.gp11.reconfigure());

    let mut main_count_down = timer.count_down();
    main_count_down.start(TIMER_RIGHT_LOOP.millis());

    let mut led = Led::new(&mut neopixel);
    let mut mode = 2;

    delay.delay_ms(2000);
    loop {
        if main_count_down.wait().is_ok() {
            match mode {
                1 => {
                    led.light_on(LedColor::Green);
                    // RECEIVE ------ ------------------------------------------------------------
                    match uart.receive() {
                        Ok(v) => {
                            if v[1] == 10 {
                                led.light_off();
                                mode = 2;
                            }
                        }
                        _ => {
                            led.light_on(LedColor::Red);
                        }
                    }
                }

                2 => {
                    // TRANSMIT ------ ------------------------------------------------------------

                    // led.light_on(LedColor::Blue);
                    let right_pins = gpios.get_active_indexes();

                    delay.delay_ms(200);
                    led.light_on(LedColor::Aqua);
                    delay = uart.send([right_pins.len() as u8, 2], delay);

                    mode = 1;
                }

                _ => {}
            }
        }

        // led.startup(TIMER_MAIN_LOOP);
        // }
    }
}

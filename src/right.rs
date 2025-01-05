#![no_std]
#![no_main]

mod utils;
use crate::utils::options::TIMER_MAIN_LOOP;
use utils::gpios::{self, Gpios};
use utils::led::{Led, RED};
use utils::options::{TIMER_LED_STARTUP, UART_SPEED};
use utils::timer::ChewTimer;

use waveshare_rp2040_zero as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry,
    gpio::{DynPinId, Pin, PullUp},
    pac,
    pio::PIOExt,
    timer::Timer,
    usb,
    watchdog::Watchdog,
    Sio,
};
use cortex_m::prelude::*;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::*;
use embedded_io::Write;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use usb_device::class_prelude::*;
use usbd_human_interface_device::prelude::*;
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

    // USB ------
    let usb_bus = UsbBusAllocator::new(usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut keyboard = UsbHidClassBuilder::new()
        .add_device(
            usbd_human_interface_device::device::keyboard::NKROBootKeyboardConfig::default(),
        )
        .build(&usb_bus);

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

    // let is_right = pins.gp12.into_floating_input().is_high().unwrap();
    // let is_left = pins.gp10.into_floating_input().is_high().unwrap();

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

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut main_count_down = timer.count_down();
    main_count_down.start(TIMER_MAIN_LOOP.millis());

    let mut chew_timer = ChewTimer::new();
    let mut led = Led::new(&mut neopixel);

    loop {
        if main_count_down.wait().is_ok() {
            // }

            // if gpio_count_down.wait().is_ok() {
            let right_pins = gpios.update_states();
            if !tx.write_all(&right_pins).is_ok() {
                // TODO Clean it if it's validated, turn off the light
                led.light_on(RED);
                continue;
            }
            // led.startup(chew_timer.ticks);
        }

        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => chew_timer.add(),
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
            };
        }
    }
}

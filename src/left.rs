#![no_std]
#![no_main]

mod keys;
mod layouts;
use core::mem::swap;

use keys::{Key, Modifiers, KC};
use layouts::LAYOUTS;
use utils::gpios::{self, Gpios};
use utils::options::{BUFFER_LENGTH, UART_SPEED};
mod utils;
use crate::utils::options::{HOLD_TIME, TIMER_MAIN_LOOP};
use usbd_human_interface_device::page::Keyboard;
use utils::led::Led;
use utils::led::{BLUE, RED};
use utils::matrix::{self, up_matrix, Matrix};
use utils::timer::ChewTimer;

use waveshare_rp2040_zero::{
    self as bsp,
    hal::gpio::{FunctionSio, SioInput},
};

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry,
    gpio::{DynPinId, FunctionSioInput, Pin, PullUp},
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
use embedded_io::Read;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use ws2812_pio::Ws2812;

#[allow(clippy::wildcard_imports)]
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::prelude::*;

use heapless::Vec;

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

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default()
            .manufacturer("f@linguenheld.fr")
            .product("RustyChew")
            .serial_number("TEST")])
        .unwrap()
        .build();

    // GPIO -----
    let mut gpios: Gpios = Gpios {
        pins: [
            [
                Some(pins.gp4.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp3.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp2.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp1.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp0.into_pull_up_input().into_dyn_pin()),
            ],
            [
                Some(pins.gp15.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp26.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp27.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp28.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp29.into_pull_up_input().into_dyn_pin()),
            ],
            [
                Some(pins.gp14.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp13.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp9.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp8.into_pull_up_input().into_dyn_pin()),
                None,
            ],
            [
                Some(pins.gp7.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp6.into_pull_up_input().into_dyn_pin()),
                Some(pins.gp5.into_pull_up_input().into_dyn_pin()),
                None,
                None,
            ],
        ],
    };

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
    let mut rx_program = pio_uart::install_rx_program(&mut pio).ok().unwrap();
    let mut rx = pio_uart::PioUartRx::new(
        pins.gp11.reconfigure(),
        sm1,
        &mut rx_program,
        UART_SPEED.Hz(),
        125.MHz(),
    )
    .enable();

    // ----------
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut main_count_down = timer.count_down();
    main_count_down.start(TIMER_MAIN_LOOP.millis());

    // let mut gpio_count_down = timer.count_down();
    // gpio_count_down.start(TIMER_GPIO.millis());

    let mut chew_timer = ChewTimer::new();
    let mut led = Led::new(&mut neopixel);

    let mut matrix_prev: Matrix = [0; 34];
    let mut matrix_cur: Matrix = [0; 34];

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    // let mut current_layout: Vec<u8> = Vec::new();
    // current_layout.push(0);
    let mut current_layout = 0;

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    let mut modifiers = Modifiers::new();
    let mut key_buffer: Vec<[Keyboard; 6], BUFFER_LENGTH> = Vec::new();

    loop {
        // TODO put in its owns timer
        // led.startup(chew_timer.ticks);
        //Poll the keys every 10ms
        if main_count_down.wait().is_ok() {
            let left_pins = gpios.update_states();

            // Matrix update ------------------------------------------------------------------
            // Read the right side ----
            let mut right_pins = [0_u8; 4];
            if !rx.read_exact(&mut right_pins).is_ok() {
                led.light_on(RED);
            }

            swap(&mut matrix_prev, &mut matrix_cur);
            up_matrix(left_pins, 'l', &chew_timer, &matrix_prev, &mut matrix_cur);
            up_matrix(right_pins, 'r', &chew_timer, &matrix_prev, &mut matrix_cur);

            if key_buffer.is_empty() {
                // Layouts ------------------------------------------------------------------
                current_layout = 0;
                for (layout_case, (case_prev, case_cur)) in LAYOUTS[current_layout]
                    .iter()
                    .zip(matrix_prev.iter().zip(matrix_cur.iter()))
                {
                    match layout_case {
                        k if (k >= &KC::A && k <= &KC::Question) => {
                            // let diff = chew_timer.diff(*case_cur);
                            // if (*case_prev == 0 && *case_cur > 0)
                            if *case_cur > 0
                                && (*case_prev == 0 || chew_timer.diff(*case_cur) >= HOLD_TIME)
                            {
                                // TODO find a way to make macro forbiden ? or to limit their speed

                                k.to_usb_code(&modifiers, &mut key_buffer);
                                // break;
                            }
                        }
                        k if (k >= &KC::HomeAltA && k <= &KC::HomeSftR) => {
                            // let diff = chew_timer.diff(*case_prev);
                            if chew_timer.diff(*case_prev) < HOLD_TIME && *case_cur == 0 {
                                k.to_usb_code(&modifiers, &mut key_buffer);
                                // break;
                            }
                        }
                        _ => {}
                    }
                }
            }

            // USB ----------------------------------------------------------------------
            let to_print = key_buffer.pop().unwrap_or([Keyboard::NoEventIndicated; 6]);

            match keyboard.device().write_report(to_print) {
                Err(UsbHidError::WouldBlock) => {}
                Err(UsbHidError::Duplicate) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write keyboard report: {:?}", e)
                }
            }
        }

        // Rename ----------------------------------------------------------------------
        // Rename ----------------------------------------------------------------------
        // Rename ----------------------------------------------------------------------
        // if gpio_count_down.wait().is_ok() {
        //     let left_pins = gpios.update_states();

        //     // Matrix update ------------------------------------------------------------------
        //     // Read the right side ----
        //     let mut right_pins = [0_u8; 4];
        //     if !rx.read_exact(&mut right_pins).is_ok() {
        //         led.light_on(RED);
        //     }

        //     swap(&mut matrix_prev, &mut matrix_cur);
        //     up_matrix(left_pins, 'l', &chew_timer, &matrix_prev, &mut matrix_cur);
        //     up_matrix(right_pins, 'r', &chew_timer, &matrix_prev, &mut matrix_cur);
        // }

        //Tick once per ms
        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => chew_timer.add(),
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
            };
        }

        if usb_dev.poll(&mut [&mut keyboard]) {
            match keyboard.device().read_report() {
                Err(UsbError::WouldBlock) => {
                    //do nothing
                }
                Err(e) => {
                    core::panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(leds) => {
                    // led_pin.set_state(PinState::from(leds.num_lock)).ok();
                    // if PinState::from(leds.num_lock) == PinState::High {
                    //     neopixel
                    //         .write(brightness(once(colors::RED.into()), 3))
                    //         .unwrap();
                    // }
                }
            }
        }
    }
}

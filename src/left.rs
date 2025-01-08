#![no_std]
#![no_main]

mod keys;
mod layouts;

use keys::{Modifiers, KC};
use layouts::LAYOUTS;
use utils::gpios::Gpios;
use utils::options::{BUFFER_LENGTH, TIMER_USB_LOOP, UART_SPEED};
mod utils;
use crate::utils::options::{HOLD_TIME, TIMER_MAIN_LOOP};
use usbd_human_interface_device::page::Keyboard;
use utils::led::{Led, OFF};
use utils::led::{BLUE, GREEN, RED};
use utils::matrix::Matrix;

use waveshare_rp2040_zero as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry, pac,
    pio::PIOExt,
    timer::Timer,
    usb,
    watchdog::Watchdog,
    Sio,
};
use cortex_m::prelude::*;
use defmt::*;
use defmt_rtt as _;
use embedded_io::Read;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use ws2812_pio::Ws2812;

#[allow(clippy::wildcard_imports)]
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::prelude::*;

use heapless::{Deque, Vec};

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
            // usbd_human_interface_device::device::keyboard::NKROBootKeyboardConfig::default(),
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
        bsp::XOSC_CRYSTAL_FREQ.Hz(),
    )
    .enable();

    // ----------
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut main_count_down = timer.count_down();
    main_count_down.start(TIMER_MAIN_LOOP.millis());

    let mut usb_count_down = timer.count_down();
    usb_count_down.start(TIMER_USB_LOOP.millis());

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    let mut current_layout = 0;
    let mut dead_layout = usize::MAX;

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    let mut led = Led::new(&mut neopixel);

    let mut matrix = Matrix::new();
    let mut mods = Modifiers::new();

    let mut key_buffer: Deque<[Keyboard; 6], BUFFER_LENGTH> = Deque::new();
    let mut previous_kc: [Keyboard; 6] = [Keyboard::NoEventIndicated; 6];

    'aaa: loop {
        // TODO put in its owns timer
        // led.startup(chew_timer.ticks);
        //Poll the keys every 10ms
        if main_count_down.wait().is_ok() {
            // Matrix update ------------------------------------------------------------
            let left_pins = gpios.update_states();
            let mut right_pins = [0_u8; 4];
            if !rx.read_exact(&mut right_pins).is_ok() {
                // TODO Clean it if it's validated, turn off the light
                led.light_on(RED);
                continue;
            }
            matrix.up(left_pins, right_pins);

            // Layouts --------------------------------------------------------------
            if dead_layout == usize::MAX {
                current_layout = 0;

                for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[current_layout]
                    .iter()
                    .enumerate()
                    .zip(matrix.prev.iter().zip(matrix.cur.iter()))
                {
                    match layout {
                        KC::Layout(number) => {
                            if *mat_cur > 0 {
                                current_layout = *number;
                                // TODO add break points
                            }
                        }
                        KC::LayDead(number) => {
                            if *mat_cur > 0 {
                                current_layout = *number;
                                // To simple, need to save a "done" state
                                dead_layout = index;
                                continue 'aaa;
                            }
                        }
                        _ => {}
                    }
                }
            }

            if current_layout == 0 {
                led.light_on(RED);
            } else if current_layout == 2 {
                led.light_on(BLUE);
            } else {
                led.light_on(OFF);
            }

            // Modifiers ------------------------------------------------------------
            // Maintain them from the matrix level instead of the layout
            // So keep their indexes
            mods.alt.0 = mods.alt.0 && matrix.cur[mods.alt.1] >= HOLD_TIME;
            mods.alt_gr.0 = mods.alt_gr.0 && matrix.cur[mods.alt_gr.1] >= HOLD_TIME;
            mods.ctrl.0 = mods.ctrl.0 && matrix.cur[mods.ctrl.1] >= HOLD_TIME;
            mods.gui.0 = mods.gui.0 && matrix.cur[mods.gui.1] >= HOLD_TIME;
            mods.shift.0 = mods.shift.0 && matrix.cur[mods.shift.1] >= HOLD_TIME;

            // Regular modifiers --
            LAYOUTS[current_layout]
                .iter()
                .zip(matrix.cur.iter())
                .enumerate()
                .filter(|(_, (&la, &mc))| la >= KC::ALT && la <= KC::SHIFT && mc > 0)
                .for_each(|(index, (layout, _))| match layout {
                    KC::ALT => mods.alt = (true, index),
                    KC::ALTGR => mods.alt_gr = (true, index),
                    KC::CTRL => mods.ctrl = (true, index),
                    KC::GUI => mods.gui = (true, index),
                    _ => mods.shift = (true, index),
                });

            // Homerow modifiers --
            LAYOUTS[current_layout]
                .iter()
                .zip(matrix.cur.iter())
                .enumerate()
                .filter(|(_, (&la, &ma))| {
                    la >= KC::HomeAltA && la <= KC::HomeSftR && ma > HOLD_TIME
                })
                .for_each(|(index, (layout, _))| match layout {
                    KC::HomeAltA | KC::HomeAltU => mods.alt = (true, index),
                    KC::HomeCtrlE | KC::HomeCtrlT => mods.ctrl = (true, index),
                    KC::HomeGuiS | KC::HomeGuiI => mods.gui = (true, index),
                    _ => mods.shift = (true, index),
                });

            // Regular keys ---------------------------------------------------------
            for (((index, layout), mat_prev), mat_cur) in LAYOUTS[current_layout]
                .iter()
                .enumerate()
                .zip(matrix.prev.iter())
                .zip(matrix.cur.iter())
                .filter(|(((index, _), _), _)| {
                    !mods.is_active(*index)
                        && !(dead_layout < usize::MAX
                            && matrix.cur[dead_layout] >= HOLD_TIME
                            && dead_layout == *index)
                })
            {
                match layout {
                    k if (k >= &KC::A && k <= &KC::Question) => {
                        if *mat_prev == 0 && *mat_cur > 0 {
                            key_buffer = k.to_usb_code(&mods, key_buffer);
                        }
                        if *mat_cur >= HOLD_TIME {
                            // k.to_usb_code(&mods, &mut key_buffer);
                        } else if *mat_prev > 0 && *mat_cur == 0 {
                            key_buffer = KC::None.to_usb_code(&mods, key_buffer);
                        }
                    }
                    k if (k >= &KC::ECircum && k <= &KC::EDiaer) => {
                        if *mat_prev == 0 && *mat_cur > 0 {
                            key_buffer = k.to_usb_code(&mods, key_buffer);
                        }
                    }
                    k if (k >= &KC::HomeAltA && k <= &KC::HomeSftR) => {
                        if *mat_prev > 0 && *mat_prev < HOLD_TIME && *mat_cur == 0 {
                            key_buffer = k.to_usb_code(&mods, key_buffer);
                        } else if *mat_prev > 0 && *mat_cur == 0 {
                            key_buffer = KC::None.to_usb_code(&mods, key_buffer);
                        }
                    }
                    _ => {}
                }
            }

            // --
            if dead_layout < usize::MAX
                && matrix.cur[dead_layout] == 0
                && matrix.cur.iter().filter(|c| **c > 0).count() > 0
            {
                dead_layout = usize::MAX;
            }
        }

        // USB ----------------------------------------------------------------------
        if usb_count_down.wait().is_ok() {
            if let Some(to_print) = key_buffer.pop_front() {
                if to_print != previous_kc {
                    previous_kc = to_print.clone();

                    match keyboard.device().write_report(to_print) {
                        Err(UsbHidError::WouldBlock) => {
                            led.light_on(BLUE);
                        }
                        Err(UsbHidError::Duplicate) => {
                            led.light_on(GREEN);
                        }
                        Ok(_) => {
                            led.light_on(OFF);
                        }
                        Err(e) => {
                            led.light_on(RED);
                            core::panic!("Failed to write keyboard report: {:?}", e)
                        }
                    }
                }
            }
        }

        //Tick once per ms
        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
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

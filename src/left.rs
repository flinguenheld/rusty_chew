#![no_std]
#![no_main]

mod keys;
mod layouts;
mod utils;

use keys::{Lay, KC};
use layouts::LAYOUTS;
use utils::gpios::Gpios;
use utils::led::{Led, LedColor};
use utils::matrix::Matrix;
use utils::modifiers::Modifiers;
use utils::options::{BUFFER_LENGTH, HOLD_TIME, TIMER_MAIN_LOOP, TIMER_USB_LOOP, UART_SPEED};

use heapless::{Deque, FnvIndexSet, Vec};

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
use usbd_human_interface_device::device::keyboard::NKROBootKeyboard;
use usbd_human_interface_device::device::mouse::{WheelMouse, WheelMouseReport};
use usbd_human_interface_device::page::Keyboard;
use usbd_human_interface_device::prelude::*;

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

    let mut rusty_chew = UsbHidClassBuilder::new()
        .add_device(
            usbd_human_interface_device::device::keyboard::NKROBootKeyboardConfig::default(),
        )
        .add_device(usbd_human_interface_device::device::mouse::WheelMouseConfig::default())
        .build(&usb_bus);

    // let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1337, 0x1985))
        .strings(&[StringDescriptors::default()
            .manufacturer("florent@linguenheld.fr")
            .product("Rusty Chew")
            .serial_number("hey")])
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

    const NB_LAYOUTS: usize = LAYOUTS.len();
    let mut layouts: Vec<Lay, NB_LAYOUTS> = Vec::new();
    let mut current_layout = 0;

    let mut matrix = Matrix::new();
    let mut mods = Modifiers::new();
    let mut homerow_history: FnvIndexSet<usize, 8> = FnvIndexSet::new();

    let mut key_buffer: Deque<[Keyboard; 6], BUFFER_LENGTH> = Deque::new();
    let mut last_printed_key: [Keyboard; 6] = [Keyboard::NoEventIndicated; 6];

    // MOUSE ----------------------------------------------------------------------------------------------
    // MOUSE ----------------------------------------------------------------------------------------------
    let mut last_mouse_buttons = 0;
    let mut mouse_report = WheelMouseReport::default();
    // MOUSE ----------------------------------------------------------------------------------------------
    // MOUSE ----------------------------------------------------------------------------------------------

    let mut led = Led::new(&mut neopixel);

    'main: loop {
        if main_count_down.wait().is_ok() {
            // Matrix update ------------------------------------------------------------
            let left_pins = gpios.update_states();
            let mut right_pins = [0_u8; 4];
            if rx.read_exact(&mut right_pins).is_err() {
                led.light_on(utils::led::LedColor::Red);
                // continue;
            } else {
                matrix.up(left_pins, right_pins);

                if matrix.prev != matrix.cur {
                    // Layouts ------------------------------------------------------------------
                    match layouts.last().unwrap_or(&Lay::Pressed(0, 0)) {
                        Lay::Dead(_, _, _) => {}
                        _ => {
                            for ((index, layout), (mat_prev, mat_cur)) in LAYOUTS[current_layout]
                                .iter()
                                .enumerate()
                                .zip(matrix.prev.iter().zip(matrix.cur.iter()))
                            {
                                match layout {
                                    KC::Layout(number) => {
                                        if *mat_cur > 0 {
                                            layouts.push(Lay::Pressed(*number, index)).ok();
                                            // break;
                                        }
                                    }
                                    KC::LayDead(number) => {
                                        if *mat_prev == 0 && *mat_cur > 0 {
                                            layouts.push(Lay::Dead(*number, index, false)).ok();

                                            // Mandatory jump to avoid its own key pressed
                                            continue 'main;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    current_layout = match layouts.last().unwrap_or(&Lay::Pressed(0, 0)) {
                        Lay::Pressed(number, _) => *number,
                        Lay::Dead(number, _, _) => *number,
                    };

                    // Modifiers ----------------------------------------------------------------
                    LAYOUTS[current_layout]
                        .iter()
                        .zip(matrix.cur.iter())
                        .enumerate()
                        .filter(|(_, (&la, &mc))| {
                            mc > 0
                                && ((la >= KC::Alt && la <= KC::Shift)
                                    || (la >= KC::HomeAltA && la <= KC::HomeSftR))
                        })
                        .for_each(|(index, (layout, _))| match layout {
                            KC::Alt => mods.alt = (true, index),
                            KC::Altgr => mods.alt_gr = (true, index),
                            KC::Ctrl => mods.ctrl = (true, index),
                            KC::Gui => mods.gui = (true, index),
                            KC::Shift => mods.shift = (true, index),

                            KC::HomeAltA | KC::HomeAltU => mods.alt = (false, index),
                            KC::HomeCtrlE | KC::HomeCtrlT => mods.ctrl = (false, index),
                            KC::HomeGuiS | KC::HomeGuiI => mods.gui = (false, index),
                            _ => mods.shift = (false, index),
                        });

                    mods.update_states(&matrix.cur);

                    // Regular keys -------------------------------------------------------------
                    for (((index, layout), mat_prev), mat_cur) in LAYOUTS[current_layout]
                        .iter()
                        .enumerate()
                        .zip(matrix.prev.iter())
                        .zip(matrix.cur.iter())
                        .filter(|(((index, _), _), _)| !mods.is_active(*index))
                    {
                        match layout {
                            k if (k >= &KC::A && k <= &KC::Yen) => {
                                // Last key is automatically repeated by the usb crate
                                if *mat_prev == 0 && *mat_cur > 0 {
                                    key_buffer = k.usb_code(&mods, key_buffer);
                                } else if *mat_prev > 0 && *mat_cur == 0 {
                                    key_buffer = KC::None.usb_code(&mods, key_buffer);
                                }
                            }
                            k if (k >= &KC::ACircum && k <= &KC::YDiaer) => {
                                if *mat_prev == 0 && *mat_cur > 0 {
                                    key_buffer = k.usb_code(&mods, key_buffer);
                                }
                            }
                            k if (k >= &KC::HomeAltA && k <= &KC::HomeSftR) => {
                                // To validate the release, the press event has to be saved in the history
                                if *mat_prev == 0 && *mat_cur > 0 {
                                    homerow_history.insert(index).ok();
                                } else if *mat_prev > 0
                                    && *mat_prev < HOLD_TIME
                                    && *mat_cur == 0
                                    && homerow_history.contains(&index)
                                {
                                    key_buffer = k.usb_code(&mods, key_buffer);
                                } else if *mat_prev > 0 && *mat_cur == 0 {
                                    key_buffer = KC::None.usb_code(&mods, key_buffer);
                                }
                            }

                            // Mouse ------------------------------------------------------------
                            k if (k >= &KC::MouseLeft && k <= &KC::MouseRight) => {
                                if *mat_cur > 0 {
                                    mouse_report = k.usb_mouse_move(
                                        mouse_report,
                                        &LAYOUTS[current_layout],
                                        &matrix.cur,
                                    );
                                }
                            }
                            k if (k >= &KC::MouseBtLeft && k <= &KC::MouseBtRight) => {
                                if *mat_cur > 0 {
                                    mouse_report.buttons |= match k {
                                        KC::MouseBtLeft => 0x1,
                                        KC::MouseBtMiddle => 0x4,
                                        _ => 0x2,
                                    }
                                } else if *mat_prev > 0 && *mat_cur == 0 {
                                    mouse_report.buttons &= match k {
                                        KC::MouseBtLeft => 0xFF - 0x1,
                                        KC::MouseBtMiddle => 0xFF - 0x4,
                                        _ => 0xFF - 0x2,
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // --
                    homerow_history
                        .retain(|&index| !(matrix.prev[index] > 0 && matrix.cur[index] == 0));
                    layouts.retain_mut(|l| match l {
                        Lay::Pressed(_, index) => matrix.cur[*index] > 0,
                        Lay::Dead(_, index, done) => {
                            if !*done {
                                if matrix.prev[*index] == 0 && matrix.cur[*index] > 0 {
                                    *done = true;
                                } else if matrix.cur[*index] == 0 {
                                    *done =
                                        matrix.cur.iter().filter(|c| **c > 0).count() > mods.nb_on()
                                } else if matrix.cur[*index] > HOLD_TIME {
                                    *done = matrix.cur.iter().filter(|c| **c > 0).count()
                                        > mods.nb_on() + 1
                                }
                            }

                            !(*done && matrix.cur[*index] < HOLD_TIME)
                        }
                    });
                    led.light_off();
                } else {
                    led.light_on(LedColor::Blue)
                }
            }

            led.startup(TIMER_MAIN_LOOP);
        }

        // USB --------------------------------------------------------------------------
        if usb_count_down.wait().is_ok() {
            if let Some(to_print) = key_buffer.pop_front() {
                if to_print != last_printed_key {
                    let keyboard = rusty_chew.device::<NKROBootKeyboard<'_, _>, _>();
                    match keyboard.write_report(to_print) {
                        // match keyboard.device().write_report(to_print) {
                        Err(UsbHidError::WouldBlock) => {
                            led.light_on(LedColor::Yellow);
                        }
                        Err(UsbHidError::Duplicate) => {
                            led.light_on(LedColor::Blue);
                        }
                        Ok(_) => {
                            last_printed_key = to_print;
                            // led.light_on(OFF);
                        }
                        Err(e) => {
                            led.light_on(LedColor::Orange);
                            core::panic!("Failed to write keyboard report: {:?}", e)
                        }
                    }
                }
            }

            if mouse_report.buttons != last_mouse_buttons
                || mouse_report.x != 0
                || mouse_report.y != 0
            {
                let mouse = rusty_chew.device::<WheelMouse<'_, _>, _>();
                match mouse.write_report(&mouse_report) {
                    Err(UsbHidError::WouldBlock) => {}
                    Ok(_) => {
                        last_mouse_buttons = mouse_report.buttons;
                        mouse_report = Default::default();
                    }
                    Err(e) => {
                        core::panic!("Failed to write mouse report: {:?}", e)
                    }
                };
            }
        }

        // Tick once per ms -------------------------------------------------------------
        if tick_count_down.wait().is_ok() {
            match rusty_chew.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
            };
        }

        if usb_dev.poll(&mut [&mut rusty_chew]) {
            match rusty_chew
                .device::<NKROBootKeyboard<'_, _>, _>()
                .read_report()
            {
                Err(UsbError::WouldBlock) => {}
                Err(e) => {
                    core::panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(_leds) => {}
            }
        }
    }
}

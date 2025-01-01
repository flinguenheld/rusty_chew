#![no_std]
#![no_main]

mod keys;
mod layouts;
use keys::{Key, Modifiers, KC};
use layouts::LAYOUTS;
mod utils;
use crate::utils::options::{HOLD_TIME, TIMER_MAIN_LOOP};
use usbd_human_interface_device::page::Keyboard;
use utils::led::{BLUE, RED};
use utils::matrix::Matrix;
use utils::timer::ChewTimer;
use utils::{led::Led, matrix::MatrixStatus};

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
    let mut gpios: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4] = [
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
    ];

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
        // 19200.Hz(),
        115_200.Hz(),
        // 921_600.Hz(),
        125.MHz(),
    )
    .enable();

    // ----------
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut input_count_down = timer.count_down();
    input_count_down.start(TIMER_MAIN_LOOP.millis());

    let mut chew_timer = ChewTimer::new();
    let mut led = Led::new(&mut neopixel);

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    let mut matrix = Matrix::new();
    // let mut current_layout: Vec<u8> = Vec::new();
    // current_layout.push(0);
    let mut current_layout = 0;

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    // let mut modifiers: [(bool, usize); 4] = [(false, 0), (false, 0), (false, 0), (false, 0)];
    let mut modifiers = Modifiers::new();

    loop {
        //Poll the keys every 10ms
        if input_count_down.wait().is_ok() {
            led.startup(chew_timer.ticks);

            // let mut pouet: Vec<[Keyboard; 5], 5> = Vec::new();
            let mut pouet: [Keyboard; 5] = [Keyboard::NoEventIndicated; 5];

            // Read the right side ----
            let mut buffer = [0_u8; 4];
            if !rx.read_exact(&mut buffer).is_ok() {
                led.light_on(RED);
            }

            // Up matrix ----
            matrix.read_left(&mut gpios, &chew_timer);
            matrix.read_right(&mut buffer, &chew_timer);

            // Current layout
            // Is it still active ?
            // Check if there is a new one ?
            // Any modificator ?

            // Layouts ------------------------------------------------------------------
            current_layout = 0;
            for (layout_case, matrix_case) in
                LAYOUTS[current_layout].iter().zip(matrix.grid.iter_mut())
            {
                match layout_case {
                    KC::LAY(number) => match matrix_case {
                        MatrixStatus::Pressed(_) | MatrixStatus::Held => {
                            current_layout = *number as usize
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            // Modifiers ----------------------------------------------------------------
            // Maintain them from the grid level instead of the layout
            modifiers.Alt.0 = matrix.grid[modifiers.Alt.1] == MatrixStatus::Held;
            modifiers.Ctrl.0 = matrix.grid[modifiers.Ctrl.1] == MatrixStatus::Held;
            modifiers.Gui.0 = matrix.grid[modifiers.Gui.1] == MatrixStatus::Held;
            modifiers.Shift.0 = matrix.grid[modifiers.Shift.1] == MatrixStatus::Held;

            let nb_key_pressed = matrix
                .grid
                .iter()
                .filter(|c| match **c {
                    MatrixStatus::Pressed(ticks) => ticks > HOLD_TIME - (HOLD_TIME / 4),
                    _ => false,
                })
                .count();

            // Loop once only for them
            for ((index, layout_case), matrix_case) in LAYOUTS[current_layout]
                .iter()
                .enumerate()
                .zip(matrix.grid.iter_mut())
            {
                match layout_case {
                    // Regular modifiers --
                    k if (k >= &KC::ALT && k <= &KC::SHIFT) => match matrix_case {
                        MatrixStatus::Pressed(_) | MatrixStatus::Held => match layout_case {
                            KC::ALT => modifiers.Alt = (true, index),
                            KC::CTRL => modifiers.Ctrl = (true, index),
                            KC::GUI => modifiers.Gui = (true, index),
                            KC::SHIFT => modifiers.Shift = (true, index),
                            _ => {}
                        },

                        _ => {}
                    },

                    // Homerow modifiers --
                    k if k >= &KC::HomeAltA && k <= &KC::HomeSftR => match matrix_case {
                        MatrixStatus::Held => match layout_case {
                            KC::HomeAltA | KC::HomeAltU => modifiers.Alt = (true, index),
                            KC::HomeCtrlE | KC::HomeCtrlT => modifiers.Ctrl = (true, index),
                            KC::HomeGuiS | KC::HomeGuiI => modifiers.Gui = (true, index),
                            KC::HomeSftN | KC::HomeSftR => modifiers.Shift = (true, index),
                            _ => {}
                        },
                        MatrixStatus::Pressed(_) => {
                            // Specific case when a homerow & another key are simultaneously pressed --
                            // TODO Set the key as Held ?
                            if nb_key_pressed > 1 {
                                match layout_case {
                                    KC::HomeAltA | KC::HomeAltU => modifiers.Alt = (true, index),
                                    KC::HomeCtrlE | KC::HomeCtrlT => modifiers.Ctrl = (true, index),
                                    KC::HomeGuiS | KC::HomeGuiI => modifiers.Gui = (true, index),
                                    KC::HomeSftN | KC::HomeSftR => modifiers.Shift = (true, index),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            if modifiers.Shift.0 {
                led.light_on(BLUE);
            } else {
                led.light_off();
            }

            // TODO: Find a way to say:
            // if a home row is pressed (not held) and some other keys -> activate the modifier

            // Keys ---------------------------------------------------------------------
            for ((index, layout_case), matrix_case) in LAYOUTS[current_layout]
                .iter()
                .enumerate()
                .zip(matrix.grid.iter_mut())
            {
                match layout_case {
                    k if (k >= &KC::A && k <= &KC::Question) => match matrix_case {
                        MatrixStatus::Pressed(ticks) => {
                            pouet = k.to_usb_code(&modifiers);
                            break;
                        }
                        MatrixStatus::Held => {
                            if !modifiers.is_active(index) {
                                pouet = k.to_usb_code(&modifiers);
                            }
                            break;
                        }
                        _ => {}
                    },

                    k if (k >= &KC::HomeAltA && k <= &KC::HomeSftR) => match matrix_case {
                        MatrixStatus::Released => {
                            pouet = k.to_usb_code(&modifiers);
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }

            // USB ----------------------------------------------------------------------
            match keyboard.device().write_report(pouet) {
                Err(UsbHidError::WouldBlock) => {}
                Err(UsbHidError::Duplicate) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write keyboard report: {:?}", e)
                }
            }
        }

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

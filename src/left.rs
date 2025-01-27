#![no_std]
#![no_main]

mod chew;
mod keys;
mod layouts;
mod utils;

use chew::Chew;
use usbd_serial::SerialPort;
use utils::gpios::Gpios;
use utils::led::{Led, LedColor};
use utils::matrix::Matrix;
use utils::options::{
    BUFFER_LENGTH, DELAY, HOLD_TIME, TIMER_MAIN_LOOP, TIMER_UART_LOOP, TIMER_USB_LOOP, UART_SPEED,
};
use utils::uart::{Mail, Uart, UartError, HR_KEYS};
use utils::{serial::*, uart};

use core::fmt::Write;
use core::panic;

use heapless::{Deque, FnvIndexSet, String, Vec};

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
// use defmt::*;
use defmt_rtt as _;
// use embedded_io::Read;

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

    // Remove ------------------------------------------------------------------------------
    // Remove ------------------------------------------------------------------------------
    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    delay.delay_ms(1000);

    let sio = Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

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

    let mut serial = SerialPort::new(&usb_bus);
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

    // TODO simplify and create the neopixel in Led ?
    let mut led = Led::new(&mut neopixel);

    // UART -----
    let mut uart = Uart::new(&mut pio, sm1, pins.gp11.reconfigure());

    // ----------
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut uart_count_down = timer.count_down();
    uart_count_down.start(TIMER_UART_LOOP.millis());

    let mut usb_count_down = timer.count_down();
    usb_count_down.start(TIMER_USB_LOOP.millis());

    let mut key_buffer: Deque<[Keyboard; 6], BUFFER_LENGTH> = Deque::new();
    let mut last_printed_key: [Keyboard; 6] = [Keyboard::NoEventIndicated; 6];

    // MOUSE ----------------------------------------------------------------------------------------------
    // MOUSE ----------------------------------------------------------------------------------------------
    let mut last_mouse_buttons = 0;
    let mut mouse_report = WheelMouseReport::default();
    // MOUSE ----------------------------------------------------------------------------------------------
    // MOUSE ----------------------------------------------------------------------------------------------

    let mut ticks: u32 = 0;
    let mut chew = Chew::new(ticks);

    loop {
        led.light_off();
        if uart_count_down.wait().is_ok() {
            // serial.write("raaaa\r\n".as_bytes()).ok();

            match uart.receive() {
                Ok(mail) => match mail.header {
                    HR_KEYS => {
                        // led.light_on(LedColor::Blue);
                        // for k in mail.values.iter() {
                        //     serial.write(num_to_str(*k as u32).as_bytes()).ok();
                        //     serial.write(" -----\r\n".as_bytes()).ok();
                        // }

                        // ---------------------------------------------------------
                        // Keyboard logic here
                        // serial
                        //     .write(time("logic here", timer.get_counter().ticks()).as_bytes())
                        //     .ok();

                        chew.update_matrix(
                            &gpios.get_left_indexes(),
                            &mail.values.iter().cloned().collect(),
                            ticks,
                        );

                        (key_buffer, mouse_report) = chew.run(key_buffer, mouse_report);
                        // if !key_buffer.is_empty() {
                        // if chew.matrix.cur[2] > 0 {
                        //     serial.write(num_to_str(chew.matrix.cur[2]).as_bytes());
                        //     serial.write("\r\n".as_bytes());
                        // }

                        // if !chew.matrix.cur[5] > 0 {
                        //     led.light_on(LedColor::Blue);
                        // } else {
                        //     // led.light_on(LedColor::Green);
                        //     led.light_off();
                        // }

                        // ---------------------------------------------------------
                        uart.send(HR_KEYS, &[], &mut delay).ok();
                    }

                    _ => {
                        serial
                            .write(time("Error !!", timer.get_counter().ticks()).as_bytes())
                            .ok();
                        led.light_on(LedColor::Red);
                    }
                },

                Err(e) => match e {
                    UartError::NothingToReadMax => {
                        serial
                            .write("Nothing to read maximum reached -----\r\n".as_bytes())
                            .ok();
                        uart.send(HR_KEYS, &[], &mut delay).ok();
                        serial.write("Send a new request -----\r\n".as_bytes()).ok();
                    }
                    err => {
                        serial.write(err.to_serial().as_bytes()).ok();
                    }
                },
            }
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
            ticks = ticks.wrapping_add(1);
            match rusty_chew.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
            };
        }

        // if !usb_dev.poll(&mut [&mut serial]) {
        //     continue;
        // }

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

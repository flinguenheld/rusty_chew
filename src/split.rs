#![no_std]
#![no_main]

mod hardware;
mod layouts;
mod options;
mod software;

use cfg_if::cfg_if;
use embedded_hal::digital::InputPin;
use hardware::{
    gpios::GpiosDirectPin,
    led::{
        Led, LedColor, LED_CAPLOCK, LED_DYNMAC_GO_WAIT, LED_DYNMAC_REC, LED_DYNMAC_REC_WAIT,
        LED_LAYOUT_FN, LED_LAYOUT_FR, LED_LEADER_KEY,
    },
    uart::{Uart, UartError, HR_KEYS, HR_LED},
};
use options::{SERIAL_ON, TIMER_SPLIT_LOOP, TIMER_UART_LOOP, TIMER_USB_LOOP};
use software::{
    chew::Chew,
    keys::{BuffCase, Buffer},
    serial_usb::{serial_write, serial_write_time, serial_write_values},
};
use usbd_serial::SerialPort;

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
use defmt_rtt as _;

use fugit::ExtU32;
use panic_probe as _;
use ws2812_pio::Ws2812;

#[allow(clippy::wildcard_imports)]
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::device::keyboard::NKROBootKeyboard;
use usbd_human_interface_device::device::mouse::{WheelMouse, WheelMouseReport};
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

    // USB --
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

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1337, 0x1985))
        .strings(&[StringDescriptors::default()
            .manufacturer("florent@linguenheld.fr")
            .product("Rusty Chew Split")
            .serial_number("00")])
        .unwrap()
        .supports_remote_wakeup(true)
        .build();

    // Side detection --
    let is_left = pins.gp10.into_pull_down_input().is_high().unwrap();

    cfg_if! {
        if #[cfg(feature = "master")] {
            let is_master = true;
        } else if #[cfg(feature = "slave")] {
            let is_master = false;
        } else {
            let is_master = pins.gpio19.into_pull_down_input().is_high().unwrap();
        }
    }

    // GPIO --
    // Here are the indexes which are used by the matrix. Gpios struct is in
    // charge of converting an active pin status into a matrix index.

    // 00  01  02  03  04    |    05  06  07  08  09
    // 10  11  12  13  14    |    15  16  17  18  19
    // 20  21  22  23        |        24  25  26  27
    //         28  29  30    |    31  32  33
    let mut gpios = GpiosDirectPin::new();
    if is_left {
        gpios.add(pins.gp4.into_pull_up_input().into_dyn_pin(), 0);
        gpios.add(pins.gp3.into_pull_up_input().into_dyn_pin(), 1);
        gpios.add(pins.gp2.into_pull_up_input().into_dyn_pin(), 2);
        gpios.add(pins.gp1.into_pull_up_input().into_dyn_pin(), 3);
        gpios.add(pins.gp0.into_pull_up_input().into_dyn_pin(), 4);
        // --
        gpios.add(pins.gp15.into_pull_up_input().into_dyn_pin(), 10);
        gpios.add(pins.gp26.into_pull_up_input().into_dyn_pin(), 11);
        gpios.add(pins.gp27.into_pull_up_input().into_dyn_pin(), 12);
        gpios.add(pins.gp28.into_pull_up_input().into_dyn_pin(), 13);
        gpios.add(pins.gp29.into_pull_up_input().into_dyn_pin(), 14);
        // --
        gpios.add(pins.gp14.into_pull_up_input().into_dyn_pin(), 20);
        gpios.add(pins.gp13.into_pull_up_input().into_dyn_pin(), 21);
        gpios.add(pins.gp9.into_pull_up_input().into_dyn_pin(), 22);
        gpios.add(pins.gp8.into_pull_up_input().into_dyn_pin(), 23);
        // --
        gpios.add(pins.gp7.into_pull_up_input().into_dyn_pin(), 28);
        gpios.add(pins.gp6.into_pull_up_input().into_dyn_pin(), 29);
        gpios.add(pins.gp5.into_pull_up_input().into_dyn_pin(), 30);
    } else {
        gpios.add(pins.gp0.into_pull_up_input().into_dyn_pin(), 5);
        gpios.add(pins.gp1.into_pull_up_input().into_dyn_pin(), 6);
        gpios.add(pins.gp2.into_pull_up_input().into_dyn_pin(), 7);
        gpios.add(pins.gp3.into_pull_up_input().into_dyn_pin(), 8);
        gpios.add(pins.gp4.into_pull_up_input().into_dyn_pin(), 9);
        // --
        gpios.add(pins.gp29.into_pull_up_input().into_dyn_pin(), 15);
        gpios.add(pins.gp28.into_pull_up_input().into_dyn_pin(), 16);
        gpios.add(pins.gp27.into_pull_up_input().into_dyn_pin(), 17);
        gpios.add(pins.gp26.into_pull_up_input().into_dyn_pin(), 18);
        gpios.add(pins.gp15.into_pull_up_input().into_dyn_pin(), 19);
        // --
        gpios.add(pins.gp8.into_pull_up_input().into_dyn_pin(), 24);
        gpios.add(pins.gp9.into_pull_up_input().into_dyn_pin(), 25);
        gpios.add(pins.gp13.into_pull_up_input().into_dyn_pin(), 26);
        gpios.add(pins.gp14.into_pull_up_input().into_dyn_pin(), 27);
        // --
        gpios.add(pins.gp5.into_pull_up_input().into_dyn_pin(), 31);
        gpios.add(pins.gp6.into_pull_up_input().into_dyn_pin(), 32);
        gpios.add(pins.gp7.into_pull_up_input().into_dyn_pin(), 33);
    }

    // Led --
    let mut neopixel = Ws2812::new(
        // The onboard NeoPixel is attached to GPIO pin #16 on the Waveshare RP2040-Zero.
        pins.neopixel.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );
    let mut led = Led::new(&mut neopixel);

    // UART --
    let mut uart = Uart::new(&mut pio, sm1, pins.gp11.reconfigure());

    // Timers --
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut uart_count_down = timer.count_down();
    uart_count_down.start(TIMER_UART_LOOP.millis());

    let mut usb_count_down = timer.count_down();
    usb_count_down.start(TIMER_USB_LOOP.millis());

    let mut split_count_down = timer.count_down();
    split_count_down.start(TIMER_SPLIT_LOOP.millis());

    // --
    let mut ticks: u32 = 0;
    let mut chew = Chew::new(ticks);
    let mut led_status;

    let mut key_buffer = Buffer::new();
    let mut last_printed_key: BuffCase = BuffCase::default();
    let mut key_buffer_tempo = 0;

    let mut mouse_report = WheelMouseReport::default();
    let mut last_mouse_buttons = 0;

    loop {
        if SERIAL_ON && !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        if uart_count_down.wait().is_ok() {
            // serial_write_time(&mut serial, "Uart loop -- ", ticks, " --\r\n");

            // --------------------------------------------------------------------------
            // ----------------------------------------------------------- UART MASTER --
            if is_master {
                match uart.receive() {
                    Ok(mail) => {
                        if mail.header == HR_KEYS {
                            serial_write_values(&mut serial, "Indexes: ", &mail.values, "\r\n");

                            // Get active indexes & combine them with the other side
                            chew.update_matrix(
                                gpios
                                    .get_active_indexes()
                                    .iter()
                                    .chain(mail.values.iter())
                                    .cloned()
                                    .collect(),
                                ticks,
                            );

                            (key_buffer, mouse_report, led_status) =
                                chew.run(key_buffer, mouse_report, ticks);

                            // Mouse report directly done here --------------------------
                            // Keyboard has its own timer to allow combinations
                            if mouse_report.buttons != last_mouse_buttons
                                || mouse_report.x != 0
                                || mouse_report.y != 0
                                || mouse_report.vertical_wheel != 0
                                || mouse_report.horizontal_wheel != 0
                            {
                                let mouse = rusty_chew.device::<WheelMouse<'_, _>, _>();
                                match mouse.write_report(&mouse_report) {
                                    Err(UsbHidError::WouldBlock) => {
                                        led.on(LedColor::Red);
                                    }
                                    Ok(_) => {
                                        last_mouse_buttons = mouse_report.buttons;
                                        mouse_report = WheelMouseReport::default();
                                    }
                                    Err(e) => {
                                        core::panic!("Failed to write mouse report: {:?}", e)
                                    }
                                };
                            }

                            // Update LED & share its state to the slave --
                            match led_status {
                                LED_LAYOUT_FR => led.on(LedColor::Aqua),
                                LED_LAYOUT_FN => led.on(LedColor::Fushia),
                                LED_LEADER_KEY => led.on(LedColor::Blue),
                                LED_CAPLOCK => led.on(LedColor::Orange),

                                LED_DYNMAC_GO_WAIT => led.blink(LedColor::Olive, 800, ticks),
                                LED_DYNMAC_REC => led.blink(LedColor::Red, 600, ticks),
                                LED_DYNMAC_REC_WAIT => led.blink(LedColor::Purple, 800, ticks),

                                _ => led.off(),
                            }

                            if uart.send(HR_LED, &[led_status], &mut delay).is_err() {
                                serial_write_time(&mut serial, "LED msg failed", ticks, " --\r\n");
                            }
                        }
                    }

                    Err(e) => match e {
                        UartError::NothingToRead => {}
                        // UartError::Capacity => led.on(LedColor::Green),
                        UartError::Header => led.on(LedColor::Blue),
                        // UartError::NotReciever => led.on(LedColor::Purple),
                        // UartError::NotTransmitter => led.on(LedColor::Red),
                        // UartError::NotComplete => led.on(LedColor::Orange),
                        UartError::NotComplete => {}
                        // UartError::Uart => led.on(LedColor::Green),
                        _ => {
                            serial_write(&mut serial, &e.to_serial());
                            led.on(LedColor::Red)
                        }
                    },
                }
            } else {
                // ----------------------------------------------------------------------
                // -------------------------------------------------------- UART SLAVE --
                match uart.receive() {
                    Ok(mail) => {
                        if mail.header == HR_LED {
                            match mail.values[0] {
                                LED_LAYOUT_FR => led.on(LedColor::Aqua),
                                LED_LAYOUT_FN => led.on(LedColor::Fushia),
                                LED_LEADER_KEY => led.on(LedColor::Blue),
                                LED_CAPLOCK => led.on(LedColor::Orange),

                                LED_DYNMAC_GO_WAIT => led.blink(LedColor::Olive, 800, ticks),
                                LED_DYNMAC_REC => led.blink(LedColor::Red, 600, ticks),
                                LED_DYNMAC_REC_WAIT => led.blink(LedColor::Purple, 800, ticks),

                                _ => led.off(),
                            }
                        }
                    }

                    Err(UartError::NothingToRead) => {}
                    // Err(UartError::Capacity) => led.on(LedColor::Green),
                    Err(UartError::Header) => led.on(LedColor::Blue),
                    // Err(UartError::NotReciever) => led.on(LedColor::Purple),
                    // Err(UartError::NotTransmitter) => led.on(LedColor::Red),
                    // Err(UartError::NotComplete) => led.on(LedColor::Orange),
                    Err(UartError::NotComplete) => {}
                    // Err(UartError::Uart) => led.on(LedColor::Green),
                    _ => led.on(LedColor::Red),
                }
            }
        }

        // ------------------------------------------------------------------------------
        // -------------------------------------------------------------------- MASTER --
        if is_master {
            // USB --
            if !SERIAL_ON && usb_count_down.wait().is_ok() && key_buffer_tempo <= ticks {
                if let Some(popped_key) = key_buffer.keys.pop_front() {
                    if popped_key != last_printed_key {
                        let keyboard = rusty_chew.device::<NKROBootKeyboard<'_, _>, _>();
                        match keyboard.write_report(popped_key.key_code.clone()) {
                            Err(UsbHidError::WouldBlock) => {
                                // Wake up --
                                if usb_dev.state() == UsbDeviceState::Suspend {
                                    usb_dev.bus().remote_wakeup();
                                    key_buffer.keys.clear();
                                } else {
                                    key_buffer.keys.push_front(popped_key).ok();
                                }
                            }
                            Err(UsbHidError::Duplicate) => {
                                led.on(LedColor::Blue);
                            }
                            Ok(_) => {
                                key_buffer_tempo = ticks.wrapping_add(popped_key.tempo);
                                last_printed_key = popped_key;
                            }
                            Err(e) => {
                                core::panic!("Failed to write keyboard report: {:?}", e)
                            }
                        }
                    }
                }
            }

            // Tick once per ms --
            if tick_count_down.wait().is_ok() {
                ticks = ticks.wrapping_add(1);
                match rusty_chew.tick() {
                    Err(UsbHidError::WouldBlock) => {}
                    Ok(_) => {}
                    Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
                };
            }

            if !SERIAL_ON && usb_dev.poll(&mut [&mut rusty_chew]) {
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
        } else {
            // --------------------------------------------------------------------------
            // ----------------------------------------------------------------- SLAVE --
            // Ticks used to blink the LED
            if tick_count_down.wait().is_ok() {
                ticks = ticks.wrapping_add(1);
            }

            // Slave is in charge of starting a new "chew loop"
            // It sends its active indexes and the master proceeds to the logic only when
            // it's able to combine the two matrix sides.
            if split_count_down.wait().is_ok()
                && uart
                    .send(HR_KEYS, &gpios.get_active_indexes(), &mut delay)
                    .is_err()
            {
                led.on(LedColor::Red);
            }
        }
    }
}

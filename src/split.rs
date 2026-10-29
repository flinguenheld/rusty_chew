#![no_std]
#![no_main]

use cfg_if::cfg_if;
use embedded_hal::digital::InputPin;
use heapless::Vec;
use rusty_chew::hardware::{
    buzzer::{Buzzer, Side, Song},
    gpios::GpiosDirectPin,
    led::{Led, LedColor},
    uart::{Uart, UartError, HEADER_MASTER, HEADER_SLAVE},
};
use rusty_chew::options::{SERIAL_ON, TIMER_LINK_REPORT, TIMER_SPLIT_LOOP, TIMER_UART_LOOP};
use rusty_chew::software::{
    chew::Chew,
    keys::Buffer,
    serial_usb::{serial_write_value, serial_write_values},
    split_mail::{
        indexes_to_mask, mail_to_status, mask_to_indexes, sides, status_to_mail, DroppedFrames,
        Link,
    },
    status::{play_status_songs, show_status_led, StatusBits},
    usb_reports::{read_keyboard, tick_hid, UsbReports},
};
use usbd_serial::SerialPort;

use waveshare_rp2040_zero::{self as bsp, hal::pwm::Slices};

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

    let (mut pio, sm0, sm1, sm2, _) = pac.PIO0.split(&mut pac.RESETS);

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

    // Side detection (releases the pin for the buzzer)--
    let mut pin10 = pins.gp10.into_pull_down_input();
    let is_left = pin10.is_high().unwrap();
    let pin10 = pin10.into_floating_input();

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

    // The slave sends its keys as a mask, both sides convert it with these arrays
    let (own_side, slave_side) = sides(is_left);

    // Buzzer (Check doc to see the assignment pin/pwm/channel)
    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);

    let mut pwm_left = pwm_slices.pwm6;
    let mut pwm_right = pwm_slices.pwm5;

    let mut buzzer = if is_left {
        pwm_left.channel_a.output_to(pins.gp12);
        Buzzer::new(&mut pwm_left, Side::Left)
    } else {
        pwm_right.channel_a.output_to(pin10);
        Buzzer::new(&mut pwm_right, Side::Right)
    };

    buzzer.add_song(Song::WelcomeC, false);

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
    let mut uart = Uart::new(
        &mut pio,
        sm1,
        sm2,
        pins.gp11.reconfigure(),
        clocks.system_clock.freq(),
    );

    // Timers --
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut uart_count_down = timer.count_down();
    uart_count_down.start(TIMER_UART_LOOP.millis());

    let mut split_count_down = timer.count_down();
    split_count_down.start(TIMER_SPLIT_LOOP.millis());

    let mut link_report_count_down = timer.count_down();
    link_report_count_down.start(TIMER_LINK_REPORT.millis());

    // Both: lost after LINK_TIMEOUT without a valid mail
    let mut link = Link::new();
    // Master: frames dropped since the last serial report
    let mut dropped = DroppedFrames::new();

    // Slave: last status bits received, None until the first STATUS frame
    let mut last_status_bits: Option<StatusBits> = None;

    // --
    let mut ticks: u32 = 0;
    let mut chew = Chew::new(ticks);

    // Master: status sent with the next request, it comes from the last chew.run.
    // Starts from the initial statuses (not 0), the buzzer can be active at startup.
    let mut status_bits = chew.status_bits();

    let mut key_buffer = Buffer::new();
    let mut mouse_report = WheelMouseReport::default();
    let mut usb_reports = UsbReports::new();

    loop {
        if SERIAL_ON && !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        if is_master {
            // --------------------------------------------------------------------------
            // ---------------------------------------------------------------- MASTER --
            if uart_count_down.wait().is_ok() {
                // Pressed keys given to the chew logic, None when it has nothing to run
                let mut active_indexes: Option<Vec<u8, 16>> = None;

                match uart.receive() {
                    Ok(mail) => {
                        // None when the mask has a bit without key (corrupted frame)
                        let slave_indexes = match mail.header {
                            HEADER_SLAVE => mask_to_indexes(&mail.values, slave_side),
                            _ => None,
                        };

                        if let Some(slave_indexes) = slave_indexes {
                            link.received(ticks);
                            dropped.answer_received();
                            serial_write_values(&mut serial, "Indexes: ", &slave_indexes, "\r\n");

                            // Get active indexes & combine them with the other side
                            // (the chew logic handles 16 keys pressed at once)
                            active_indexes = Some(
                                gpios
                                    .get_active_indexes()
                                    .iter()
                                    .chain(slave_indexes.iter())
                                    .take(16)
                                    .cloned()
                                    .collect(),
                            );
                        } else if mail.header == HEADER_SLAVE {
                            dropped.bad_masks += 1;
                        }
                    }

                    Err(e) => match e {
                        UartError::NothingToRead => {}
                        // A corrupted frame is only a lost cycle, the next request comes in 5 ms
                        UartError::Crc => dropped.crc += 1,
                        UartError::Header => led.on(LedColor::Blue),
                        // UartError::NotComplete => led.on(LedColor::Orange),
                        UartError::NotComplete => {}
                    },
                }

                // Lost link: every key is released once, so none stays stuck, then the logic
                // waits for the slave. The master doesn't type alone, without the slave's
                // layer keys & modifiers its own keys would give other characters.
                if link.just_lost(ticks) {
                    active_indexes = Some(Vec::new());
                    serial_write_value(&mut serial, "Link lost at tick:", ticks, "\r\n");
                }

                if let Some(active_indexes) = active_indexes {
                    chew.update_matrix(active_indexes, ticks);

                    (key_buffer, mouse_report) = chew.run(key_buffer, mouse_report, ticks);

                    // Mouse report directly done here, the keyboard goes through the queue --
                    usb_reports.write_mouse(
                        rusty_chew.device::<WheelMouse<'_, _>, _>(),
                        &mut mouse_report,
                        &mut led,
                    );

                    // LED & songs, the slave gets the same state with the next request --
                    let last_bits = status_bits;
                    status_bits = chew.status_bits();

                    // Red until the slave is back, the logic doesn't run in between
                    if link.is_lost() {
                        led.on(LedColor::Red);
                    } else {
                        show_status_led(&mut led, status_bits, ticks);
                    }
                    play_status_songs(&mut buzzer, Some(last_bits), status_bits);
                }

                // The master is the only one to start an exchange: it sends its status as a
                // request and the slave answers with its keys, so both never talk at once.
                // Still sent while the link is lost, the slave only comes back by answering.
                if split_count_down.wait().is_ok() {
                    dropped.request_sent();
                    uart.send(HEADER_MASTER, status_to_mail(status_bits), &mut delay);
                }

                if link_report_count_down.wait().is_ok() {
                    dropped.report(&mut serial);
                }

                buzzer.sing(ticks);
            }

            // USB --
            if !SERIAL_ON {
                usb_reports.write_keyboard(
                    rusty_chew.device::<NKROBootKeyboard<'_, _>, _>(),
                    &usb_dev,
                    &mut key_buffer,
                    &mut led,
                    ticks,
                );
            }

            // Tick once per ms --
            if tick_count_down.wait().is_ok() {
                ticks = ticks.wrapping_add(1);
                tick_hid(&mut rusty_chew);
            }

            if !SERIAL_ON && usb_dev.poll(&mut [&mut rusty_chew]) {
                read_keyboard(rusty_chew.device::<NKROBootKeyboard<'_, _>, _>());
            }
        } else {
            // --------------------------------------------------------------------------
            // ----------------------------------------------------------------- SLAVE --
            if uart_count_down.wait().is_ok() {
                match uart.receive() {
                    Ok(mail) => {
                        if mail.header == HEADER_MASTER {
                            link.received(ticks);

                            // The changes are found by comparing with the last state, so a lost
                            // frame only delays them. The first state is taken without songs.
                            let status_bits = mail_to_status(&mail.values);
                            show_status_led(&mut led, status_bits, ticks);
                            play_status_songs(&mut buzzer, last_status_bits, status_bits);
                            last_status_bits = Some(status_bits);

                            // Answer the request with the pressed keys, the slave only talks
                            // when the master asks
                            let mask = indexes_to_mask(&gpios.get_active_indexes(), own_side);
                            uart.send(HEADER_SLAVE, mask, &mut delay);
                        }
                    }

                    Err(UartError::NothingToRead) => {}
                    // The master sends its status again in the next cycle
                    Err(UartError::Crc) => {}
                    Err(UartError::Header) => led.on(LedColor::Blue),
                    // Err(UartError::NotComplete) => led.on(LedColor::Orange),
                    Err(UartError::NotComplete) => {}
                }

                // Lost link: red until the master is back, its first state is then taken
                // without songs (the changes in between are unknown)
                if link.just_lost(ticks) {
                    last_status_bits = None;
                    led.on(LedColor::Red);
                }

                buzzer.sing(ticks);
            }

            // Ticks used to blink the LED
            if tick_count_down.wait().is_ok() {
                ticks = ticks.wrapping_add(1);
            }
        }
    }
}

#![no_std]
#![no_main]

use cfg_if::cfg_if;
use embedded_hal::digital::InputPin;
use rusty_chew::hardware::{
    buzzer::{Buzzer, Side, Song},
    gpios::GpiosDirectPin,
    led::{Led, LedColor},
    uart::{Uart, UartError, HR_KEYS, HR_STATUS},
};
use rusty_chew::options::{SERIAL_ON, TIMER_LINK_REPORT, TIMER_SPLIT_LOOP, TIMER_UART_LOOP, TIMER_USB_LOOP};
use rusty_chew::software::{
    chew::Chew,
    keys::{BuffCase, Buffer},
    serial_usb::{serial_write, serial_write_time, serial_write_value, serial_write_values},
    split_mail::{
        changed_status_song, indexes_to_mask, is_buzzer_active, mask_to_indexes, show_status_led,
        sides, statuses_to_bits,
    },
    status::{Status, Statuses},
};
use usbd_serial::SerialPort;

use waveshare_rp2040_zero::{
    self as bsp,
    hal::pwm::{SliceId, Slices},
};

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

    let mut buz_left = if is_left {
        let mut pwm = pwm_slices.pwm6;
        pwm.channel_a.output_to(pins.gp12);
        Some(Buzzer::new(pwm))
    } else {
        None
    };
    let mut buz_right = if !is_left {
        let mut pwm = pwm_slices.pwm5;
        pwm.channel_a.output_to(pin10);
        Some(Buzzer::new(pwm))
    } else {
        None
    };

    sing(&mut buz_left, &mut buz_right, Song::WelcomeC, false);

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
        pins.gp11.reconfigure(),
        clocks.system_clock.freq(),
    );

    // Timers --
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut uart_count_down = timer.count_down();
    uart_count_down.start(TIMER_UART_LOOP.millis());

    let mut usb_count_down = timer.count_down();
    usb_count_down.start(TIMER_USB_LOOP.millis());

    let mut split_count_down = timer.count_down();
    split_count_down.start(TIMER_SPLIT_LOOP.millis());

    let mut link_report_count_down = timer.count_down();
    link_report_count_down.start(TIMER_LINK_REPORT.millis());

    // Frames dropped by the master since the last report
    let mut crc_errors: u32 = 0;
    let mut bad_masks: u32 = 0;

    // Slave: last status bits received, None until the first STATUS frame
    let mut last_status_bits: Option<u8> = None;

    // --
    let mut ticks: u32 = 0;
    let mut chew = Chew::new(ticks);
    let mut statuses = Statuses::new();

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
                        // None when the mask has a bit without key (corrupted frame)
                        let slave_indexes = match mail.header {
                            HR_KEYS => mask_to_indexes(&mail.values, slave_side),
                            _ => None,
                        };

                        if let Some(slave_indexes) = slave_indexes {
                            serial_write_values(&mut serial, "Indexes: ", &slave_indexes, "\r\n");

                            // Get active indexes & combine them with the other side
                            // (the chew logic handles 16 keys pressed at once)
                            chew.update_matrix(
                                gpios
                                    .get_active_indexes()
                                    .iter()
                                    .chain(slave_indexes.iter())
                                    .take(16)
                                    .cloned()
                                    .collect(),
                                ticks,
                            );

                            (key_buffer, mouse_report, statuses) =
                                chew.run(key_buffer, mouse_report, statuses, ticks);

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

                            // Update LED & BUZZER, the slave gets the same state --
                            let status_bits = statuses_to_bits(&statuses);
                            let buzzer_active = is_buzzer_active(status_bits);
                            show_status_led(&mut led, status_bits, ticks);

                            // Enabled before the songs and disabled after them, so the
                            // buzzer toggle song can be heard
                            if buzzer_active {
                                active_buzzer(&mut buz_left, &mut buz_right, true);
                            }

                            if statuses.layout_fr == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::TwoDF, false);
                            } else if statuses.layout_fr == Status::SwitchOff {
                                sing(&mut buz_left, &mut buz_right, Song::TwoDF, true);
                            } else if statuses.layout_fn == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::NotifC, false);
                            } else if statuses.layout_fn == Status::SwitchOff {
                                sing(&mut buz_left, &mut buz_right, Song::NotifC, true);
                            } else if statuses.leader_key == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::NotifA, false);
                            } else if statuses.caplock == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::StartupG, false);
                            } else if statuses.caplock == Status::SwitchOff {
                                sing(&mut buz_left, &mut buz_right, Song::StartupG, true);
                            } else if statuses.dynmac_go_waitkey == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::NotifF, false);
                            } else if statuses.dynmac_rec_inprogess == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::AlertD, false);
                            } else if statuses.dynmac_rec_waitkey == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::AlertE, false);
                            } else if statuses.buzzer_activation == Status::SwitchOn {
                                sing(&mut buz_left, &mut buz_right, Song::StartupA, false);
                            } else if statuses.buzzer_activation == Status::SwitchOff {
                                sing(&mut buz_left, &mut buz_right, Song::StartupA, true);
                            }

                            if !buzzer_active {
                                active_buzzer(&mut buz_left, &mut buz_right, false);
                            }

                            if uart
                                .send(HR_STATUS, [status_bits, 0, 0], &mut delay)
                                .is_err()
                            {
                                serial_write_time(&mut serial, "STAT msg failed", ticks, " --\r\n");
                            }
                        } else if mail.header == HR_KEYS {
                            bad_masks += 1;
                        }
                    }

                    Err(e) => match e {
                        UartError::NothingToRead => {}
                        // A corrupted frame is only a lost cycle, the slave sends again in 5 ms
                        UartError::Crc => crc_errors += 1,
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

                if link_report_count_down.wait().is_ok() {
                    serial_write_value(&mut serial, "Dropped frames / s - CRC:", crc_errors, "");
                    serial_write_value(&mut serial, "| Bad masks:", bad_masks, "\r\n");
                    crc_errors = 0;
                    bad_masks = 0;
                }
            } else {
                // ----------------------------------------------------------------------
                // -------------------------------------------------------- UART SLAVE --
                match uart.receive() {
                    Ok(mail) => {
                        if mail.header == HR_STATUS {
                            let status_bits = mail.values[0];
                            let buzzer_active = is_buzzer_active(status_bits);
                            show_status_led(&mut led, status_bits, ticks);

                            if buzzer_active {
                                active_buzzer(&mut buz_left, &mut buz_right, true);
                            }

                            // The changes are found by comparing with the last state, so a lost
                            // frame only delays them. The first state is taken without songs.
                            if let Some((song, reverse)) = last_status_bits
                                .and_then(|last| changed_status_song(last, status_bits))
                            {
                                sing(&mut buz_left, &mut buz_right, song, reverse);
                            }

                            if !buzzer_active {
                                active_buzzer(&mut buz_left, &mut buz_right, false);
                            }
                            last_status_bits = Some(status_bits);
                        }
                    }

                    Err(UartError::NothingToRead) => {}
                    // The master sends its status again in the next cycle
                    Err(UartError::Crc) => {}
                    Err(UartError::Header) => led.on(LedColor::Blue),
                    // Err(UartError::NotReciever) => led.on(LedColor::Purple),
                    // Err(UartError::NotTransmitter) => led.on(LedColor::Red),
                    // Err(UartError::NotComplete) => led.on(LedColor::Orange),
                    Err(UartError::NotComplete) => {}
                    // Err(UartError::Uart) => led.on(LedColor::Green),
                    _ => led.on(LedColor::Red),
                }
            }

            if let Some(ref mut buzzer) = buz_left {
                buzzer.sing(ticks);
            } else if let Some(ref mut buzzer) = buz_right {
                buzzer.sing(ticks);
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
            // It sends its pressed keys mask and the master proceeds to the logic only when
            // it's able to combine the two matrix sides.
            if split_count_down.wait().is_ok() {
                let mask = indexes_to_mask(&gpios.get_active_indexes(), own_side);

                if uart.send(HR_KEYS, mask, &mut delay).is_err() {
                    led.on(LedColor::Red);
                }
            }
        }
    }
}

fn sing<IL: SliceId, IR: SliceId>(
    buzzer_left: &mut Option<Buzzer<IL>>,
    buzzer_right: &mut Option<Buzzer<IR>>,
    song: Song,
    reverse: bool,
) {
    if let Some(buzz) = buzzer_left {
        buzz.add_song(song, Side::Left, reverse);
    } else if let Some(buzz) = buzzer_right {
        buzz.add_song(song, Side::Right, reverse);
    }
}

fn active_buzzer<IL: SliceId, IR: SliceId>(
    buzzer_left: &mut Option<Buzzer<IL>>,
    buzzer_right: &mut Option<Buzzer<IR>>,
    active: bool,
) {
    if let Some(buzz) = buzzer_left {
        buzz.set_active(active);
    } else if let Some(buzz) = buzzer_right {
        buzz.set_active(active);
    }
}

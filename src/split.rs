#![no_std]
#![no_main]

mod hardware;
mod layouts;
mod options;
mod software;

use cfg_if::cfg_if;
use embedded_hal::digital::InputPin;
use hardware::{
    buzzer::{Buzzer, Side, Song},
    gpios::GpiosDirectPin,
    led::{Led, LedColor},
    uart::{Uart, UartError, FRAME_VALUES, HR_KEYS, HR_STATUS},
};
use heapless::Vec;
use options::{SERIAL_ON, TIMER_LINK_REPORT, TIMER_SPLIT_LOOP, TIMER_UART_LOOP, TIMER_USB_LOOP};
use software::{
    chew::Chew,
    keys::{BuffCase, Buffer},
    serial_usb::{serial_write, serial_write_time, serial_write_value, serial_write_values},
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

// Status --
// Bit of each status in the byte sent to the slave, the lowest bit set gives the LED
const BIT_LAYOUT_FR: u8 = 0;
const BIT_LAYOUT_FN: u8 = 1;
const BIT_LEADER_KEY: u8 = 2;
const BIT_CAPLOCK: u8 = 3;

const BIT_DYNMAC_GO_WAIT: u8 = 4;
const BIT_DYNMAC_REC: u8 = 5;
const BIT_DYNMAC_REC_WAIT: u8 = 6;

const BIT_BUZZER: u8 = 7;

// Keys mask --
// Matrix indexes of each half, bit n of the mask is the key at position n
const LEFT_INDEXES: [u8; 17] = [
    0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 20, 21, 22, 23, 28, 29, 30,
];
const RIGHT_INDEXES: [u8; 17] = [
    5, 6, 7, 8, 9, 15, 16, 17, 18, 19, 24, 25, 26, 27, 31, 32, 33,
];

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
    let (own_side, slave_side) = if is_left {
        (&LEFT_INDEXES, &RIGHT_INDEXES)
    } else {
        (&RIGHT_INDEXES, &LEFT_INDEXES)
    };

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
                            let buzzer_active = status_bits & (1 << BIT_BUZZER) != 0;
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
                            let buzzer_active = status_bits & (1 << BIT_BUZZER) != 0;
                            show_status_led(&mut led, status_bits, ticks);

                            if buzzer_active {
                                active_buzzer(&mut buz_left, &mut buz_right, true);
                            }

                            // The changes are found by comparing with the last state, so a lost
                            // frame only delays them. The first state is taken without songs.
                            if let Some(last) = last_status_bits {
                                let changed = last ^ status_bits;

                                // One song per frame, the lowest bit first (same as the master)
                                if let Some((song, reverse)) = (0..8)
                                    .filter(|bit| changed & (1 << bit) != 0)
                                    .find_map(|bit| status_song(bit, status_bits & (1 << bit) != 0))
                                {
                                    sing(&mut buz_left, &mut buz_right, song, reverse);
                                }
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

/// Slave's pressed keys -> mask, bit n is the key at position n of its side array.
fn indexes_to_mask(indexes: &[u8], side: &[u8; 17]) -> [u8; FRAME_VALUES] {
    let mut mask = 0u32;
    for index in indexes {
        if let Some(bit) = side.iter().position(|i| i == index) {
            mask |= 1 << bit;
        }
    }
    [mask as u8, (mask >> 8) as u8, (mask >> 16) as u8]
}

/// Mask -> slave's pressed keys.
/// A bit set above the 17 keys can only come from a corrupted frame which passed the CRC.
fn mask_to_indexes(mask: &[u8; FRAME_VALUES], side: &[u8; 17]) -> Option<Vec<u8, 17>> {
    let mask = u32::from_le_bytes([mask[0], mask[1], mask[2], 0]);
    if mask >> side.len() != 0 {
        return None;
    }

    Some(
        side.iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, index)| *index)
            .collect(),
    )
}

/// Master's statuses -> one byte, a bit is set while its status is on.
fn statuses_to_bits(statuses: &Statuses) -> u8 {
    [
        (BIT_LAYOUT_FR, &statuses.layout_fr),
        (BIT_LAYOUT_FN, &statuses.layout_fn),
        (BIT_LEADER_KEY, &statuses.leader_key),
        (BIT_CAPLOCK, &statuses.caplock),
        (BIT_DYNMAC_GO_WAIT, &statuses.dynmac_go_waitkey),
        (BIT_DYNMAC_REC, &statuses.dynmac_rec_inprogess),
        (BIT_DYNMAC_REC_WAIT, &statuses.dynmac_rec_waitkey),
        (BIT_BUZZER, &statuses.buzzer_activation),
    ]
    .iter()
    .filter(|(_, status)| matches!(status, Status::On | Status::SwitchOn))
    .fold(0, |bits, (bit, _)| bits | (1 << bit))
}

/// LED of the lowest status bit set (the buzzer bit has no color).
fn show_status_led(led: &mut Led, bits: u8, ticks: u32) {
    // trailing_zeros gives 8 when no bit is set
    match (bits & !(1 << BIT_BUZZER)).trailing_zeros() as u8 {
        BIT_LAYOUT_FR => led.on(LedColor::Aqua),
        BIT_LAYOUT_FN => led.on(LedColor::Fushia),
        BIT_LEADER_KEY => led.on(LedColor::Blue),
        BIT_CAPLOCK => led.on(LedColor::Orange),

        BIT_DYNMAC_GO_WAIT => led.blink(LedColor::Olive, 800, ticks),
        BIT_DYNMAC_REC => led.blink(LedColor::Red, 600, ticks),
        BIT_DYNMAC_REC_WAIT => led.blink(LedColor::Purple, 800, ticks),

        _ => led.off(),
    }
}

/// Song played by the slave when a status bit changes (song, reverse), None for no song.
fn status_song(bit: u8, switched_on: bool) -> Option<(Song, bool)> {
    match (bit, switched_on) {
        (BIT_LAYOUT_FR, on) => Some((Song::StartupB, !on)),
        (BIT_LAYOUT_FN, on) => Some((Song::NotifC, !on)),
        (BIT_LEADER_KEY, true) => Some((Song::NotifA, false)),
        (BIT_CAPLOCK, on) => Some((Song::StartupG, !on)),

        (BIT_DYNMAC_GO_WAIT, true) => Some((Song::NotifF, false)),
        (BIT_DYNMAC_REC, true) => Some((Song::AlertD, false)),
        (BIT_DYNMAC_REC_WAIT, true) => Some((Song::AlertE, false)),

        (BIT_BUZZER, on) => Some((Song::StartupA, !on)),
        _ => None,
    }
}

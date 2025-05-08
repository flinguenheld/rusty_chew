#![no_std]
#![no_main]

mod hardware;
mod layouts;
mod options;
mod software;

use hardware::{
    buzzer::{Buzzer, Song},
    gpios::GpiosMono,
    led::{Led, LedColor},
};
use options::{SERIAL_ON, TIMER_MONO_LOOP, TIMER_USB_LOOP};
use software::{
    chew::Chew,
    keys::{BuffCase, Buffer},
    status::{Status, Statuses},
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

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

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

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1338, 0x1985))
        .strings(&[StringDescriptors::default()
            .manufacturer("florent@linguenheld.fr")
            .product("Rusty Chew Mono")
            .serial_number("01")])
        .unwrap()
        .supports_remote_wakeup(true)
        .build();

    // GPIO --
    let mut gpios = GpiosMono {
        rows: [
            pins.gp5.into_pull_down_input().into_dyn_pin(),
            pins.gp6.into_pull_down_input().into_dyn_pin(),
            pins.gp7.into_pull_down_input().into_dyn_pin(),
            pins.gp8.into_pull_down_input().into_dyn_pin(),
        ],

        columns: [
            pins.gp28.into_push_pull_output().into_dyn_pin(),
            pins.gp27.into_push_pull_output().into_dyn_pin(),
            pins.gp26.into_push_pull_output().into_dyn_pin(),
            pins.gp15.into_push_pull_output().into_dyn_pin(),
            pins.gp14.into_push_pull_output().into_dyn_pin(),
            pins.gp4.into_push_pull_output().into_dyn_pin(),
            pins.gp3.into_push_pull_output().into_dyn_pin(),
            pins.gp2.into_push_pull_output().into_dyn_pin(),
            pins.gp1.into_push_pull_output().into_dyn_pin(),
            pins.gp0.into_push_pull_output().into_dyn_pin(),
        ],
    };

    // Buzzer (Check doc to see the assignment pin/pwm/channel)
    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm = pwm_slices.pwm6;
    pwm.channel_b.output_to(pins.gp29);
    let mut buzzer = Buzzer::new(pwm);

    buzzer.add_song(Song::EMinor_Up);
    // buzzer.add_song(Song::EMinor_down);

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

    // Timers --
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut mono_count_down = timer.count_down();
    mono_count_down.start(TIMER_MONO_LOOP.millis());

    let mut usb_count_down = timer.count_down();
    usb_count_down.start(TIMER_USB_LOOP.millis());

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
            led.on(LedColor::Red);
            continue;
        }

        if mono_count_down.wait().is_ok() {
            // serial_write_value(&mut serial, "max duty: ", max_duty, " <-");

            let active_indexes = gpios.get_active_indexes(&mut delay);
            chew.update_matrix(active_indexes, ticks);
            (key_buffer, mouse_report, statuses) =
                chew.run(key_buffer, mouse_report, statuses, ticks);

            if statuses.layout_fr == Status::On {
                led.on(LedColor::Aqua);
            } else if statuses.layout_fn == Status::On {
                led.on(LedColor::Fushia);
            } else if statuses.leader_key == Status::On {
                led.on(LedColor::Blue);
            } else if statuses.caplock == Status::On {
                led.on(LedColor::Orange);
            } else if statuses.dynmac_go_waitkey == Status::On {
                led.blink(LedColor::Olive, 800, ticks);
            } else if statuses.dynmac_rec_inprogess == Status::On {
                led.blink(LedColor::Red, 600, ticks);
            } else if statuses.dynmac_rec_waitkey == Status::On {
                led.blink(LedColor::Purple, 800, ticks);
            } else {
                led.off();
            }

            if statuses.layout_fr == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.layout_fr == Status::SwitchOff {
                buzzer.add_song(Song::EMinor_down);
            } else if statuses.layout_fn == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.layout_fn == Status::SwitchOff {
                buzzer.add_song(Song::EMinor_down);
            } else if statuses.leader_key == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.caplock == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.caplock == Status::SwitchOff {
                buzzer.add_song(Song::EMinor_down);
            } else if statuses.dynmac_go_waitkey == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.dynmac_rec_inprogess == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            } else if statuses.dynmac_rec_waitkey == Status::SwitchOn {
                buzzer.add_song(Song::EMinor_Up);
            }

            // Mouse report directly done here ------------------------------------------
            // Keyboard has its own timer two allow combinations
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

            buzzer.sing(ticks);
        }

        // USB --------------------------------------------------------------------------
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
                                led.on(LedColor::Red);
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

        // Tick once per ms -------------------------------------------------------------
        if tick_count_down.wait().is_ok() {
            ticks = ticks.wrapping_add(1);
            match rusty_chew.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
            };
        }

        // if !usb_dev.poll(&mut [&mut _serial]) {
        //     continue;
        // }

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
    }
}

#![no_std]
#![no_main]

use rusty_chew::hardware::{
    buzzer::{Buzzer, Side, Song},
    gpios::GpiosMono,
    led::{Led, LedColor},
};
use rusty_chew::options::{SERIAL_ON, TIMER_MONO_LOOP};
use rusty_chew::software::{
    chew::Chew,
    keys::Buffer,
    status::{play_status_songs, show_status_led},
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
    let mut buzzer = Buzzer::new(&mut pwm, Side::Both);

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

    // Timers --
    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut mono_count_down = timer.count_down();
    mono_count_down.start(TIMER_MONO_LOOP.millis());

    // --
    let mut ticks: u32 = 0;
    let mut chew = Chew::new(ticks);
    // Status bits of the last run, the songs are played for their changes
    let mut last_status_bits = chew.status_bits();

    let mut key_buffer = Buffer::new();
    let mut mouse_report = WheelMouseReport::default();
    let mut usb_reports = UsbReports::new();

    loop {
        if SERIAL_ON && !usb_dev.poll(&mut [&mut serial]) {
            led.on(LedColor::Red);
            continue;
        }

        if mono_count_down.wait().is_ok() {
            let active_indexes = gpios.get_active_indexes(&mut delay);
            chew.update_matrix(active_indexes, ticks);
            (key_buffer, mouse_report) = chew.run(key_buffer, mouse_report, ticks);

            // LED & buzzer, from the statuses' changes since the last run --
            let bits = chew.status_bits();
            show_status_led(&mut led, bits, ticks);
            play_status_songs(&mut buzzer, Some(last_status_bits), bits);
            last_status_bits = bits;

            // Mouse report directly done here, the keyboard goes through the queue --
            usb_reports.write_mouse(
                rusty_chew.device::<WheelMouse<'_, _>, _>(),
                &mut mouse_report,
                &mut led,
            );

            buzzer.sing(ticks);
        }

        // USB --------------------------------------------------------------------------
        if !SERIAL_ON {
            usb_reports.write_keyboard(
                rusty_chew.device::<NKROBootKeyboard<'_, _>, _>(),
                &usb_dev,
                &mut key_buffer,
                &mut led,
                ticks,
            );
        }

        // Tick once per ms -------------------------------------------------------------
        if tick_count_down.wait().is_ok() {
            ticks = ticks.wrapping_add(1);
            tick_hid(&mut rusty_chew);
        }

        if !SERIAL_ON && usb_dev.poll(&mut [&mut rusty_chew]) {
            read_keyboard(rusty_chew.device::<NKROBootKeyboard<'_, _>, _>());
        }
    }
}

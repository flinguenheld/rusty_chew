#![no_std]
#![no_main]

mod led;

use waveshare_rp2040_zero as bsp;

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
use embedded_io::Write;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use usb_device::class_prelude::*;
use usbd_human_interface_device::prelude::*;
use ws2812_pio::Ws2812;

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

    // GPIO -----
    let mut row_0 = [
        pins.gp0.into_pull_up_input().into_dyn_pin(),
        pins.gp1.into_pull_up_input().into_dyn_pin(),
        pins.gp2.into_pull_up_input().into_dyn_pin(),
        pins.gp3.into_pull_up_input().into_dyn_pin(),
        pins.gp4.into_pull_up_input().into_dyn_pin(),
    ];

    let mut row_1 = [
        pins.gp29.into_pull_up_input().into_dyn_pin(),
        pins.gp28.into_pull_up_input().into_dyn_pin(),
        pins.gp27.into_pull_up_input().into_dyn_pin(),
        pins.gp26.into_pull_up_input().into_dyn_pin(),
        pins.gp15.into_pull_up_input().into_dyn_pin(),
    ];

    let mut row_2 = [
        pins.gp5.into_pull_up_input().into_dyn_pin(),
        pins.gp6.into_pull_up_input().into_dyn_pin(),
        pins.gp7.into_pull_up_input().into_dyn_pin(),
        // --
        pins.gp8.into_pull_up_input().into_dyn_pin(),
        pins.gp9.into_pull_up_input().into_dyn_pin(),
        pins.gp13.into_pull_up_input().into_dyn_pin(),
        pins.gp14.into_pull_up_input().into_dyn_pin(),
    ];

    // let is_right = pins.gp12.into_floating_input().is_high().unwrap();
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
    let mut tx_program = pio_uart::install_tx_program(&mut pio).ok().unwrap();
    let mut tx = pio_uart::PioUartTx::new(
        pins.gp11.reconfigure(),
        sm1,
        &mut tx_program,
        19200.Hz(),
        125.MHz(),
    )
    .enable();

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut input_count_down = timer.count_down();
    input_count_down.start(10.millis());

    let mut startup = led::LedStartup::new(&timer, &mut neopixel);

    loop {
        // Poll the keys every 10ms
        if input_count_down.wait().is_ok() {
            startup.run();

            tx.write(&get_keys(&mut row_0, &mut row_1, &mut row_2)).ok();
        }

        // Tick once per ms
        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to process keyboard tick: {:?}", e)
                }
            };
        }
    }
}

fn get_keys(
    row_0: &mut [Pin<DynPinId, FunctionSioInput, PullUp>],
    row_1: &mut [Pin<DynPinId, FunctionSioInput, PullUp>],
    row_2: &mut [Pin<DynPinId, FunctionSioInput, PullUp>],
) -> [u8; 3] {
    let mut output = [0_u8; 3];

    for k in row_0.iter_mut() {
        output[0] <<= 1;
        if k.is_low().unwrap() {
            output[0] |= 1;
        }
    }
    for k in row_1.iter_mut() {
        output[1] <<= 1;
        if k.is_low().unwrap() {
            output[1] |= 1;
        }
    }
    for k in row_2.iter_mut() {
        output[2] <<= 1;
        if k.is_low().unwrap() {
            output[2] |= 1;
        }
    }

    output
}

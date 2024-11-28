#![no_std]
#![no_main]
mod colors;

use core::iter::once;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use waveshare_rp2040_zero as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry, pac,
    pio::PIOExt,
    timer::Timer,
    watchdog::Watchdog,
    Sio,
};
use smart_leds::{brightness, SmartLedsWrite};
use ws2812_pio::Ws2812;

#[entry]
fn main() -> ! {
    info!("Program start");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // Configure the addressable LED
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut ws = Ws2812::new(
        // The onboard NeoPixel is attached to GPIO pin #16 on the Waveshare RP2040-Zero.
        pins.neopixel.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    loop {
        info!("on!");
        // GRB ???
        ws.write(brightness(once(colors::RED.into()), 3)).unwrap();
        delay.delay_ms(250);
        ws.write(brightness(once(colors::GREEN.into()), 3)).unwrap();
        delay.delay_ms(250);
        ws.write(brightness(once(colors::BLUE.into()), 3)).unwrap();
        delay.delay_ms(250);
        ws.write(brightness(once(colors::OFF.into()), 3)).unwrap();
        delay.delay_ms(1000);
        info!("off!");
    }
}

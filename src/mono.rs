#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;
use embassy_rp::{
    bind_interrupts, dma,
    peripherals::{DMA_CH0, PIO0},
    pio::{self, Pio},
    pio_programs::ws2812::PioWs2812,
};
use embassy_time::Timer;
use rusty_chew::software::led::{self, color};
use {defmt_rtt as _, panic_probe as _};

#[cfg(feature = "zero")]
type LedOrder = embassy_rp::pio_programs::ws2812::Grb;
#[cfg(not(feature = "zero"))]
type LedOrder = embassy_rp::pio_programs::ws2812::Rgb;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

// Declare async tasks
#[embassy_executor::task]
async fn test_led(mut led: PioWs2812<'static, PIO0, 0, 1, LedOrder>) {
    loop {
        for per in (0..101).rev().step_by(10) {
            led.write(&[color(led::RED, per)]).await;
            Timer::after_millis(30).await;
        }
        for per in (0..101).rev().step_by(10) {
            led.write(&[color(led::GREEN, per)]).await;
            Timer::after_millis(30).await;
        }
        for per in (0..101).rev().step_by(10) {
            led.write(&[color(led::BLUE, per)]).await;
            Timer::after_millis(30).await;
        }
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Clocks, PLLs, watchdog, timer: all done by init()

    let p = embassy_rp::init(Default::default());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);
    let program = PioWs2812Program::new(&mut common);
    let led = PioWs2812::with_color_order(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_16, &program);

    spawner.spawn(test_led(led).unwrap());
}

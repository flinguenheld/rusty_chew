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
use rusty_chew::software::led::{self, Led};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

// Declare async tasks
#[embassy_executor::task]
async fn test_led(mut led: Led) {
    loop {
        led.set(led::RED, 200)
            .fade(300)
            .await
            .set(led::BLUE, 100)
            .fade(500)
            .await
            .off()
            .fade(1000)
            .await;

        Timer::after_millis(2000).await;
        // led.set(led::GREEN, 150).flash(500).await;

        // Timer::after_millis(2000).await;
        // led.set(led::BLUE, 150).flash(500).await;

        // Timer::after_millis(2000).await;
        // led.set(led::BLUE, 150).go().await.off().go().await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Clocks, PLLs, watchdog, timer: all done by init()

    let p = embassy_rp::init(Default::default());

    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let program = PioWs2812Program::new(&mut common);
    let led = Led::new(PioWs2812::with_color_order(
        &mut common,
        sm0,
        p.DMA_CH0,
        Irqs,
        p.PIN_16,
        &program,
    ));

    spawner.spawn(test_led(led).unwrap());
}

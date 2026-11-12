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
use embassy_rp::{peripherals::USB, usb};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};
use rusty_chew::led::{self, Led};
use rusty_chew::options::SERIAL_PACKET_SIZE;
use rusty_chew::usb::device::UsbBuffers;
use rusty_chew::usb::tasks::{serial_rx_run, serial_tx_run};
use rusty_chew::usb::types::{UsbDriver, SERIAL, USB_BUFFERS};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
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

/// Echo each message received on the serial.
#[embassy_executor::task]
async fn test_serial() {
    loop {
        let message = SERIAL.read().await;
        SERIAL.print(b"message received! ----------------\n");
        SERIAL.print(&message);
        SERIAL.print(b"\n");
    }
}

/// Answer the PC forever: enumeration, setup requests, suspend & resume.
/// Woken by USBCTRL_IRQ. Without it, the PC never sees the device.
#[embassy_executor::task]
async fn usb_run(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
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

    // 1. Driver: the hardware, woken by USBCTRL_IRQ
    let driver = usb::Driver::new(p.USB, Irqs);

    // 2. Device config: what the host sees (lsusb)
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("florent@linguenheld.net");
    config.product = Some("Chew embassy");
    config.serial_number = Some("1506");

    // 3. Buffers the builder writes into (filled once, 'static)
    let b = USB_BUFFERS.init(UsbBuffers {
        config_descriptor: [0; 256],
        bos_descriptor: [0; 256],
        control_buf: [0; 64],
        state: State::new(),
    });

    let mut builder = Builder::new(
        driver,
        config,
        &mut b.config_descriptor,
        &mut b.bos_descriptor,
        &mut [],
        &mut b.control_buf,
    );

    // 4. The class adds its interfaces & endpoints to the builder
    let serial_class = CdcAcmClass::new(
        &mut builder,
        &mut b.state,
        SERIAL_PACKET_SIZE as u16,
    );

    let (sender, receiver) = serial_class.split();

    // 5. Build the device: no more changes to the descriptors
    let usb = builder.build();

    // 6. The device and the serial run in their own tasks
    spawner.spawn(usb_run(usb).unwrap());
    spawner.spawn(serial_rx_run(receiver).unwrap());
    spawner.spawn(serial_tx_run(sender).unwrap());
    spawner.spawn(test_serial().unwrap());
}

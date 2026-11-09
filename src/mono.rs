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
use rusty_chew::usb::device::{UsbBuffers, UsbDriver, USB_BUFFERS};
use rusty_chew::usb::serial::UsbSerial;
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

/// Answer the PC forever: enumeration, setup requests, suspend & resume.
/// Woken by USBCTRL_IRQ. Without it, the PC never sees the device.
#[embassy_executor::task]
async fn usb_run(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
}

/// Send "hello" every second, to check the serial.
#[embassy_executor::task]
async fn test_serial(mut usb_serial: UsbSerial) {
    loop {
        usb_serial.read().await;

        while let Some(line) = usb_serial.get_line() {
            // usb_serial.set(b"You said something!!!\n").write().await;

            usb_serial.set(&line).write().await;
            usb_serial.set(b"\n").write().await;
        }

        // let blah = b"hello to you darling, that's a wonderfull day to go for a stroll, and then we will eat some piece of cake if you want to.\n";
        // usb_serial.set(blah).write().await;
        // let blah = b"Oh yes that would be great! I secretly hoped you said that..:)\n";
        // usb_serial.set(blah).write().await;
        // usb_serial.set(b"Hello hello!\n").write().await;
        // Timer::after_millis(50).await;
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
    // QUESTION: rename that ? I suppose there will be a lot of classes?
    let class = CdcAcmClass::new(
        &mut builder,
        &mut b.state,
        SERIAL_PACKET_SIZE as u16,
    );

    // 5. Build the device: no more changes to the descriptors
    let usb = builder.build();

    // 6. The device and the serial run in their own tasks
    spawner.spawn(usb_run(usb).unwrap());
    spawner.spawn(test_serial(UsbSerial::new(class)).unwrap());
}

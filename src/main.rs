#![no_std]
#![no_main]

mod led;

use waveshare_rp2040_zero as bsp;

use alloc::vec::Vec;
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    entry,
    gpio::{DynPinId, FunctionSioInput, Pin, PinState, PullUp},
    pac,
    pio::PIOExt,
    timer::Timer,
    uart::{DataBits, StopBits, UartConfig},
    usb,
    watchdog::Watchdog,
    Sio,
};
use cfg_if::cfg_if;
use embedded_io::{Read, Write};
use pio_uart::PioUart;

use cortex_m::prelude::*;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::*;
use fugit::{ExtU32, RateExtU32};

use panic_probe as _;

#[allow(clippy::wildcard_imports)]
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::page::Keyboard;
use usbd_human_interface_device::prelude::*;

use smart_leds::{brightness, SmartLedsWrite};
use ws2812_pio::Ws2812;

// use heapless::Vec;

extern crate alloc;

use core::ptr::addr_of_mut;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    {
        // Embedded-alloc - Init heap
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

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
    let (mut pio, sm0, sm1, sm2, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut neopixel = Ws2812::new(
        // The onboard NeoPixel is attached to GPIO pin #16 on the Waveshare RP2040-Zero.
        pins.neopixel.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    // USB
    let usb_bus = UsbBusAllocator::new(usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    info!("aaa");

    let mut keyboard = UsbHidClassBuilder::new()
        .add_device(
            usbd_human_interface_device::device::keyboard::NKROBootKeyboardConfig::default(),
        )
        .build(&usb_bus);

    // https://pid.codes
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default()
            .manufacturer("f@linguenheld.fr")
            .product("RustyChew")
            .serial_number("TEST")])
        .unwrap()
        .build();

    //GPIO pins
    let mut keys: [Pin<DynPinId, FunctionSioInput, PullUp>; 17] = [
        pins.gp4.into_pull_up_input().into_dyn_pin(),
        pins.gp3.into_pull_up_input().into_dyn_pin(),
        pins.gp2.into_pull_up_input().into_dyn_pin(),
        pins.gp1.into_pull_up_input().into_dyn_pin(),
        pins.gp0.into_pull_up_input().into_dyn_pin(),
        // --
        pins.gp15.into_pull_up_input().into_dyn_pin(),
        pins.gp26.into_pull_up_input().into_dyn_pin(),
        pins.gp27.into_pull_up_input().into_dyn_pin(),
        pins.gp28.into_pull_up_input().into_dyn_pin(),
        pins.gp29.into_pull_up_input().into_dyn_pin(),
        // --
        pins.gp14.into_pull_up_input().into_dyn_pin(),
        pins.gp13.into_pull_up_input().into_dyn_pin(),
        pins.gp9.into_pull_up_input().into_dyn_pin(),
        pins.gp8.into_pull_up_input().into_dyn_pin(),
        // --
        pins.gp7.into_pull_up_input().into_dyn_pin(),
        pins.gp6.into_pull_up_input().into_dyn_pin(),
        pins.gp5.into_pull_up_input().into_dyn_pin(),
    ];

    let is_left = pins.gp10.into_floating_input().is_high().unwrap();

    // Test UART ----------------------------------------------------------------------------------------
    // Test UART ----------------------------------------------------------------------------------------

    // Split PIO0 to be able to program it
    // let (mut pio, sm0, sm1, _sm2, _sm3) = pac.PIO0.split(&mut pac.RESETS);
    // Program RX and TX programs into PIO0
    let mut rx_program = pio_uart::install_rx_program(&mut pio).ok().unwrap();

    cfg_if::cfg_if! {
        if #[cfg(feature="slave")] {
             let mut tx_program = pio_uart::install_tx_program(&mut pio).ok().unwrap();
             let mut tx = pio_uart::PioUartTx::new(
                 pins.gp11.reconfigure(),
                 sm2,
                 &mut tx_program,
                 19200.Hz(),
                 125.MHz(),
             )
             .enable();
        } else {
             let mut rx = pio_uart::PioUartRx::new(
                 pins.gp11.reconfigure(),
                 sm1,
                 &mut rx_program,
                 19200.Hz(),
                 125.MHz(),
             )
             .enable();
        }
    }

    // Test UART ----------------------------------------------------------------------------------------
    // Test UART ----------------------------------------------------------------------------------------

    // let mut input_count_down = timer.count_down();
    // input_count_down.start(10.millis());

    let mut input_count_down = timer.count_down();
    input_count_down.start(10.millis());

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut startup = led::LedStartup::new(&timer, &mut neopixel);

    loop {
        // pouet.to
        //Poll the keys every 10ms
        if input_count_down.wait().is_ok() {
            startup.run();

            cfg_if::cfg_if! {
                if #[cfg(feature="slave")] {
                    // Find a way to convert into a vec of Keyboard
                    // let mut arr: [u8; 5] = [0, 0, 0, 0, 0];
                    let mut arr = 0;
                    if keys[0].is_low().unwrap(){
                        // arr[0]=1;
                        arr= 6;
                    }
                    tx.write(&arr).ok();
                    // tx.write(b"Hello, UART over PIO!").ok();
                } else {
                    let mut pouet = get_keys(&mut keys);

                    // let mut buffer = [0u8; 5];
                    let mut buffer= [0_u8;3];
                    rx.read(&mut buffer).ok();


                    // if buffer[0]==1{
                    if buffer[0] & 0b000010000 ==   0b000010000 {
                        pouet.push(Keyboard::J);
                    }
                    if buffer[0] & 0b000001000 ==   0b000001000 {
                        pouet.push(Keyboard::M);
                    }
                    if buffer[0] & 0b000000010 ==   0b000000010 {
                        pouet.push(Keyboard::D);
                    }
                    if buffer[0] & 0b000000100 ==   0b000000100 {
                        pouet.push(Keyboard::Y);
                    }
                    if buffer[0] & 0b000000001 ==   0b000000001 {
                        pouet.push(Keyboard::W);
                    }

                    // Find a way to convert into a vec of Keyboard
                    // match keyboard.device().write_report(pouet) {
                    //     Err(UsbHidError::WouldBlock) => {}
                    //     Err(UsbHidError::Duplicate) => {}
                    //     Ok(_) => {}
                    //     Err(e) => {
                    //         core::panic!("Failed to write keyboard report: {:?}", e)
                    //     }
                    // };
                }
            }
        }

        //Tick once per ms
        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to process keyboard tick: {:?}", e)
                }
            };
        }

        if usb_dev.poll(&mut [&mut keyboard]) {
            match keyboard.device().read_report() {
                Err(UsbError::WouldBlock) => {
                    //do nothing
                }
                Err(e) => {
                    core::panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(leds) => {
                    // led_pin.set_state(PinState::from(leds.num_lock)).ok();
                    // if PinState::from(leds.num_lock) == PinState::High {
                    //     neopixel
                    //         .write(brightness(once(colors::RED.into()), 3))
                    //         .unwrap();
                    // }
                }
            }
        }
    }
}

fn get_keys(keys: &mut [Pin<DynPinId, FunctionSioInput, PullUp>]) -> Vec<Keyboard> {
    let mut output = Vec::new();
    if keys[0].is_low().unwrap() {
        // Keyboard::Q
        output.push(Keyboard::A);
        output.push(Keyboard::B);
        output.push(Keyboard::C);
        // if cfg!("left")
        #[cfg(feature = "master")]
        {
            output.push(Keyboard::C);
            output.push(Keyboard::H);
            output.push(Keyboard::E);
            output.push(Keyboard::W);
        }
    } else {
        // output.push(Keyboard::NoEventIndicated);
    }

    output
}

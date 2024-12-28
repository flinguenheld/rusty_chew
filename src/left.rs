#![no_std]
#![no_main]

mod keys;
use keys::Key;
mod utils;
use utils::matrix::Matrix;
use utils::timer::ChewTimer;
use utils::{led::LedStartup, matrix::MatrixStatus};
use Keyboard as K;

use waveshare_rp2040_zero::{
    self as bsp,
    hal::gpio::{FunctionSio, SioInput},
};

use alloc::vec::Vec;
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
use embedded_io::Read;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use ws2812_pio::Ws2812;

#[allow(clippy::wildcard_imports)]
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_human_interface_device::page::Keyboard;
use usbd_human_interface_device::prelude::*;

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

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default()
            .manufacturer("f@linguenheld.fr")
            .product("RustyChew")
            .serial_number("TEST")])
        .unwrap()
        .build();

    // GPIO -----
    let mut gpios: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4] = [
        [
            Some(pins.gp4.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp3.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp2.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp1.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp0.into_pull_up_input().into_dyn_pin()),
        ],
        [
            Some(pins.gp15.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp26.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp27.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp28.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp29.into_pull_up_input().into_dyn_pin()),
        ],
        [
            Some(pins.gp14.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp13.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp9.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp8.into_pull_up_input().into_dyn_pin()),
            None,
        ],
        [
            Some(pins.gp7.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp6.into_pull_up_input().into_dyn_pin()),
            Some(pins.gp5.into_pull_up_input().into_dyn_pin()),
            None,
            None,
        ],
    ];

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
    let mut rx_program = pio_uart::install_rx_program(&mut pio).ok().unwrap();
    let mut rx = pio_uart::PioUartRx::new(
        pins.gp11.reconfigure(),
        sm1,
        &mut rx_program,
        // 19200.Hz(),
        115200.Hz(),
        125.MHz(),
    )
    .enable();

    // ----------
    let mut input_count_down = timer.count_down();
    input_count_down.start(10.millis());

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(1.millis());

    let mut chew_timer = ChewTimer::new();
    let mut startup = LedStartup::new(&mut neopixel);

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    #[rustfmt::skip]
    let layouts: [[Key; 34]; 2] = [[
        Key::Std(K::Q), Key::Std(K::C), Key::Std(K::O), Key::Std(K::P), Key::Std(K::V),      Key::Std(K::J), Key::Std(K::M), Key::Std(K::D), Key::Std(K::Y), Key::Std(K::W),
        Key::Std(K::A), Key::Std(K::S), Key::Std(K::E), Key::Std(K::N), Key::Std(K::F),      Key::Std(K::L), Key::Std(K::R), Key::Std(K::T), Key::Std(K::I), Key::Std(K::U),
        Key::Std(K::Z), Key::Std(K::X), Key::Std(K::E), Key::Std(K::B),                                      Key::Std(K::H), Key::Std(K::G), Key::Std(K::E), Key::Std(K::K),
                                        Key::Std(K::A), Key::Std(K::A), Key::Std(K::A),      Key::Std(K::ReturnEnter), Key::Layout(1), Key::Std(K::A)
    ],
    [
        Key::Std(K::Q), Key::Std(K::C), Key::Std(K::O), Key::Std(K::P), Key::Std(K::V),      Key::Std(K::J), Key::Std(K::Keyboard7), Key::Std(K::Keyboard8), Key::Std(K::Keyboard9), Key::Std(K::W),
        Key::Std(K::A), Key::Std(K::S), Key::Std(K::E), Key::Std(K::N), Key::Std(K::F),      Key::Std(K::L), Key::Std(K::Keyboard3), Key::Std(K::Keyboard5), Key::Std(K::Keyboard6), Key::Std(K::U),
        Key::Std(K::Z), Key::Std(K::X), Key::Std(K::E), Key::Std(K::B),                                      Key::Std(K::Keyboard1), Key::Std(K::Keyboard2), Key::Std(K::Keyboard3), Key::Std(K::K),
                                        Key::Std(K::A), Key::Std(K::A), Key::Std(K::A),      Key::Std(K::ReturnEnter), Key::None, Key::Std(K::A)
    ]
    
    
    ];

    let mut matrix = Matrix::new();
    let mut current_layout: Vec<u8> = Vec::new();
    current_layout.push(0);

    // TEST LAYOUT ---------------------------------------------------------------------------------
    // TEST LAYOUT ---------------------------------------------------------------------------------

    loop {
        //Poll the keys every 10ms
        if input_count_down.wait().is_ok() {
            startup.run(chew_timer.ticks);

            let mut pouet = Vec::new();

            // Read the right ----
            let mut buffer = [0_u8; 4];
            rx.read(&mut buffer).ok();

            // Up the matrix ----
            matrix.read_left(&mut gpios, &chew_timer);
            matrix.read_right(&mut buffer, &chew_timer);

            // Current layout
            // Is it still active ?
            // Check if there is a new one ?
            // Any modificator ?

            // Loop and push all keys

            // for (row, status_row) in layouts.iter().zip(matrix.grid.iter_mut()) {
            //     for (key, key_status) in layouts.iter().zip(matrix.grid.iter_mut()) {}
            //     match key_status {
            //         MatrixStatus::Wait(ticks) => {}
            //         _ => {}
            //     }
            // }

            if matrix.grid[0] == MatrixStatus::Pressed {
                pouet.push(Keyboard::Q);
            }
            if matrix.grid[1] == MatrixStatus::Pressed {
                pouet.push(Keyboard::C);
            }
            if matrix.grid[2] == MatrixStatus::Pressed {
                pouet.push(Keyboard::O);
            }
            if matrix.grid[3] == MatrixStatus::Pressed {
                pouet.push(Keyboard::P);
            }
            if matrix.grid[4] == MatrixStatus::Pressed {
                pouet.push(Keyboard::V);
            }

            if matrix.grid[10] == MatrixStatus::Pressed {
                pouet.push(Keyboard::A);
            }
            if matrix.grid[11] == MatrixStatus::Pressed {
                pouet.push(Keyboard::S);
            }
            if matrix.grid[12] == MatrixStatus::Pressed {
                pouet.push(Keyboard::E);
            }
            if matrix.grid[13] == MatrixStatus::Pressed {
                pouet.push(Keyboard::N);
            }
            if matrix.grid[14] == MatrixStatus::Pressed {
                pouet.push(Keyboard::F);
            }

            if matrix.grid[20] == MatrixStatus::Pressed {
                pouet.push(Keyboard::Z);
            }
            if matrix.grid[21] == MatrixStatus::Pressed {
                pouet.push(Keyboard::X);
            }
            if matrix.grid[22] == MatrixStatus::Pressed {
                pouet.push(Keyboard::E);
            }
            if matrix.grid[23] == MatrixStatus::Pressed {
                pouet.push(Keyboard::B);
            }

            if matrix.grid[28] == MatrixStatus::Pressed {
                pouet.push(Keyboard::A);
            }
            if matrix.grid[29] == MatrixStatus::Pressed {
                pouet.push(Keyboard::B);
            }
            if matrix.grid[30] == MatrixStatus::Pressed {
                pouet.push(Keyboard::C);
            }

            // --------------------------------------
            // --------------------------------------
            // --------------------------------------
            // --------------------------------------
            // --------------------------------------
            // --------------------------------------
            if matrix.grid[5] == MatrixStatus::Pressed {
                pouet.push(Keyboard::J);
            }
            if matrix.grid[6] == MatrixStatus::Pressed {
                pouet.push(Keyboard::M);
            }
            if matrix.grid[7] == MatrixStatus::Pressed {
                pouet.push(Keyboard::D);
            }
            if matrix.grid[8] == MatrixStatus::Pressed {
                pouet.push(Keyboard::Y);
            }
            if matrix.grid[9] == MatrixStatus::Pressed {
                pouet.push(Keyboard::W);
            }

            // --------------------------------------
            // --------------------------------------
            if matrix.grid[15] == MatrixStatus::Pressed {
                pouet.push(Keyboard::L);
            }
            if matrix.grid[16] == MatrixStatus::Pressed {
                pouet.push(Keyboard::R);
            }
            if matrix.grid[17] == MatrixStatus::Pressed {
                pouet.push(Keyboard::T);
            }
            if matrix.grid[18] == MatrixStatus::Pressed {
                pouet.push(Keyboard::I);
            }
            if matrix.grid[19] == MatrixStatus::Pressed {
                pouet.push(Keyboard::U);
            }

            // --------------------------------------
            // --------------------------------------
            if matrix.grid[24] == MatrixStatus::Pressed {
                pouet.push(Keyboard::H);
            }
            if matrix.grid[25] == MatrixStatus::Pressed {
                pouet.push(Keyboard::G);
            }
            if matrix.grid[26] == MatrixStatus::Pressed {
                pouet.push(Keyboard::E);
            }
            if matrix.grid[27] == MatrixStatus::Pressed {
                pouet.push(Keyboard::K);
            }

            // --------------------------------------
            // --------------------------------------
            if matrix.grid[31] == MatrixStatus::Pressed {
                pouet.push(Keyboard::A);
            }
            if matrix.grid[32] == MatrixStatus::Pressed {
                pouet.push(Keyboard::B);
            }
            if matrix.grid[33] == MatrixStatus::Pressed {
                pouet.push(Keyboard::C);
            }

            // Find a way to convert into a vec of Keyboard
            match keyboard.device().write_report(pouet) {
                Err(UsbHidError::WouldBlock) => {}
                Err(UsbHidError::Duplicate) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write keyboard report: {:?}", e)
                }
            };
        }

        //Tick once per ms
        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => chew_timer.add(),
                Err(e) => core::panic!("Failed to process keyboard tick: {:?}", e),
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

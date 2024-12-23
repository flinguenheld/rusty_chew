use waveshare_rp2040_zero::{self as bsp};

use bsp::hal::{
    gpio::{bank0::Gpio16, FunctionPio0, Pin, PullDown},
    pac::PIO0,
    pio::SM0,
    timer::CountDown,
    timer::Timer,
};
use core::iter::once;
use cortex_m::prelude::*;
use defmt_rtt as _;
use fugit::ExtU32;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

pub const OFF: [u8; 3] = [0, 0, 0];
// pub const GREEN: [u8; 3] = [255, 0, 0];
// pub const RED: [u8; 3] = [0, 255, 0];
// pub const BLUE: [u8; 3] = [0, 0, 255];

type Neopixel<'a> = Ws2812<PIO0, SM0, CountDown<'a>, Pin<Gpio16, FunctionPio0, PullDown>>;

pub struct LedStartup<'a> {
    n: u8,
    on: bool,
    countdown: CountDown<'a>,
    neopixel: &'a mut Neopixel<'a>,
}

impl LedStartup<'_> {
    pub fn new<'a>(timer: &'a Timer, neopixel: &'a mut Neopixel<'a>) -> LedStartup<'a> {
        let mut countdown = timer.count_down();
        countdown.start(10000.millis());

        LedStartup {
            n: 0,
            on: true,
            countdown,
            neopixel,
        }
    }
    pub fn run(&mut self) {
        if self.on {
            self.neopixel
                .write(brightness(once(wheel(self.n)), 3))
                .unwrap();
            self.n = self.n.wrapping_add(1);

            if self.countdown.wait().is_ok() {
                self.on = false;
                self.neopixel
                    .write(brightness(once(OFF.into()), 3))
                    .unwrap();
            }
        }
    }
}

/// Wheel from rp2040 hal example
/// Convert a number from `0..=255` to an GRB color triplet.
pub fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        // No green in this sector - red and blue only
        (255 - (wheel_pos * 3), 0, wheel_pos * 3).into()
    } else if wheel_pos < 170 {
        // No red in this sector - green and blue only
        wheel_pos -= 85;
        (0, wheel_pos * 3, 255 - (wheel_pos * 3)).into()
    } else {
        // No blue in this sector - red and green only
        wheel_pos -= 170;
        (wheel_pos * 3, 255 - (wheel_pos * 3), 0).into()
    }
}

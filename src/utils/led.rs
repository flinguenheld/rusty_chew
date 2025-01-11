use waveshare_rp2040_zero::{self as bsp};

use bsp::hal::{
    gpio::{bank0::Gpio16, FunctionPio0, Pin, PullDown},
    pac::PIO0,
    pio::SM0,
    timer::CountDown,
};
use core::iter::once;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

use super::options::TIMER_LED_STARTUP;

pub const OFF: [u8; 3] = [0, 0, 0];
pub const GREEN: [u8; 3] = [255, 0, 0];
pub const RED: [u8; 3] = [0, 255, 0];
pub const BLUE: [u8; 3] = [0, 0, 255];
pub const ORANGE: [u8; 3] = [0, 100, 100];

type Neopixel<'a> = Ws2812<PIO0, SM0, CountDown<'a>, Pin<Gpio16, FunctionPio0, PullDown>>;

pub struct Led<'a> {
    n: u8,
    on: bool,
    startup_countdown: u32,
    neopixel: &'a mut Neopixel<'a>,
}

impl Led<'_> {
    pub fn new<'a>(neopixel: &'a mut Neopixel<'a>) -> Led<'a> {
        Led {
            n: 0,
            on: true,
            startup_countdown: TIMER_LED_STARTUP,
            neopixel,
        }
    }
    pub fn startup(&mut self, ticks: u32) {
        if self.on {
            self.neopixel
                .write(brightness(once(wheel(self.n)), 3))
                .unwrap();
            self.n = self.n.wrapping_add(1);

            if ticks > self.startup_countdown {
                self.light_off();
                self.on = false;
            }
        }
    }

    pub fn light_on(&mut self, color: [u8; 3]) {
        self.neopixel
            .write(brightness(once(color.into()), 3))
            .unwrap();
    }
    pub fn light_off(&mut self) {
        self.neopixel
            .write(brightness(once(OFF.into()), 3))
            .unwrap();
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

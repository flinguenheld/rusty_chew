use core::iter::once;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use waveshare_rp2040_zero::hal::{
    gpio::{bank0::Gpio16, FunctionPio0, Pin, PullDown},
    pac::PIO0,
    pio::SM0,
    timer::CountDown,
};
use ws2812_pio::Ws2812;

use super::options::TIMER_LED_STARTUP;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum LedColor {
    Green,
    Red,
    Blue,
    Orange,
    Gray,
    Maroon,
    Yellow,
    Olive,
    Lime,
    Aqua,
    Teal,
    Navy,
    Fushia,
    Purple,
}

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
            startup_countdown: 0,
            neopixel,
        }
    }
    pub fn startup(&mut self, time: u32) {
        if self.on {
            self.startup_countdown += time;
            if self.startup_countdown > TIMER_LED_STARTUP {
                self.light_off();
                self.on = false;
            } else {
                self.neopixel
                    .write(brightness(once(wheel(self.n)), 3))
                    .unwrap();
                self.n = self.n.wrapping_add(1);
            }
        }
    }

    pub fn light_on(&mut self, color: LedColor) {
        self.neopixel
            .write(brightness(
                once(
                    match color {
                        LedColor::Green => [255, 0, 0],
                        LedColor::Red => [0, 255, 0],
                        LedColor::Blue => [0, 0, 255],
                        LedColor::Orange => [128, 255, 0],
                        LedColor::Gray => [128, 128, 128],
                        LedColor::Maroon => [0, 128, 0],
                        LedColor::Yellow => [255, 255, 0],
                        LedColor::Olive => [128, 128, 0],
                        LedColor::Lime => [128, 0, 0],
                        LedColor::Aqua => [255, 0, 255],
                        LedColor::Teal => [128, 0, 128],
                        LedColor::Navy => [0, 0, 128],
                        LedColor::Fushia => [0, 255, 255],
                        LedColor::Purple => [0, 128, 128],
                    }
                    .into(),
                ),
                3,
            ))
            .unwrap();
    }
    pub fn light_off(&mut self) {
        self.neopixel
            .write(brightness(once([0, 0, 0].into()), 3))
            .unwrap();
    }
}

/// Wheel from rp2040 hal example
/// Convert a number from `0..=255` to an GRB color triplet.
fn wheel(mut wheel_pos: u8) -> RGB8 {
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

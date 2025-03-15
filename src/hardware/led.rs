use cfg_if::cfg_if;
use core::iter::once;
use smart_leds::{brightness, SmartLedsWrite};
use waveshare_rp2040_zero::hal::{
    gpio::{bank0::Gpio16, FunctionPio0, Pin, PullDown},
    pac::PIO0,
    pio::SM0,
    timer::CountDown,
};
use ws2812_pio::Ws2812;

// Status --
pub const LED_LAYOUT_FR: u8 = 1;
pub const LED_LAYOUT_FN: u8 = 2;
pub const LED_LEADER_KEY: u8 = 3;
pub const LED_CAPLOCK: u8 = 4;

pub const LED_DYNMAC_REC: u8 = 5;
pub const LED_DYNMAC_GO_WAIT: u8 = 6;
pub const LED_DYNMAC_REC_WAIT: u8 = 7;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
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
    None,
}

type Neopixel<'a> = Ws2812<PIO0, SM0, CountDown<'a>, Pin<Gpio16, FunctionPio0, PullDown>>;

pub struct Led<'a> {
    neopixel: &'a mut Neopixel<'a>,
    next_blinking_step: u32,
    last_blinking_color: LedColor,
}

impl Led<'_> {
    pub fn new<'a>(neopixel: &'a mut Neopixel<'a>) -> Led<'a> {
        Led {
            neopixel,
            next_blinking_step: 0,
            last_blinking_color: LedColor::None,
        }
    }

    pub fn blink(&mut self, color: LedColor, step_ms: u32, ticks: u32) {
        if self.last_blinking_color != color || self.next_blinking_step == 0 {
            self.next_blinking_step = ticks.wrapping_add(step_ms);
        }

        if self.next_blinking_step > ticks {
            self.on(color);
        } else if self.next_blinking_step + step_ms > ticks {
            self.on(LedColor::None);
        } else {
            self.next_blinking_step = ticks.wrapping_add(step_ms);
        }

        self.last_blinking_color = color;
    }

    pub fn on(&mut self, color: LedColor) {
        cfg_if! {

            // RP2040-zero is GRB while Gemini is RGB -_-'
            if #[cfg(feature = "zero")] {
                self.neopixel
                    .write(brightness(
                        once(
                            match color {
                                LedColor::Green  => [ 255 ,   0 ,   0 ],
                                LedColor::Red    => [   0 , 255 ,   0 ],
                                LedColor::Blue   => [   0 ,   0 , 255 ],
                                LedColor::Orange => [ 128 , 255 ,   0 ],
                                LedColor::Gray   => [ 128 , 128 , 128 ],
                                LedColor::Maroon => [   0 , 128 ,   0 ],
                                LedColor::Yellow => [ 255 , 255 ,   0 ],
                                LedColor::Olive  => [ 128 , 128 ,   0 ],
                                LedColor::Lime   => [ 128 ,   0 ,   0 ],
                                LedColor::Aqua   => [ 255 ,   0 , 255 ],
                                LedColor::Teal   => [ 128 ,   0 , 128 ],
                                LedColor::Navy   => [   0 ,   0 , 128 ],
                                LedColor::Fushia => [   0 , 255 , 255 ],
                                LedColor::Purple => [   0 , 128 , 128 ],
                                LedColor::None   => [   0 ,   0 ,   0 ],
                            }
                            .into(),
                        ),
                        3,
                    ))
                    .unwrap();
            } else {

                self.neopixel
                    .write(brightness(
                        once(
                            match color {
                                LedColor::Green  => [   0 , 255 ,   0 ],
                                LedColor::Red    => [ 255 ,   0 ,   0 ],
                                LedColor::Blue   => [   0 ,   0 , 255 ],
                                LedColor::Orange => [ 255 , 128 ,   0 ],
                                LedColor::Gray   => [ 128 , 128 , 128 ],
                                LedColor::Maroon => [ 128 ,   0 ,   0 ],
                                LedColor::Yellow => [ 255 , 255 ,   0 ],
                                LedColor::Olive  => [ 128 , 128 ,   0 ],
                                LedColor::Lime   => [   0 , 128 ,   0 ],
                                LedColor::Aqua   => [   0 , 255 , 255 ],
                                LedColor::Teal   => [   0 , 128 , 128 ],
                                LedColor::Navy   => [   0 ,   0 , 128 ],
                                LedColor::Fushia => [ 255 ,   0 , 255 ],
                                LedColor::Purple => [ 128 ,   0 , 128 ],
                                LedColor::None   => [   0 ,   0 ,   0 ],
                            }
                            .into(),
                        ),
                        20,
                    ))
                    .unwrap();
             }
        }
    }

    pub fn off(&mut self) {
        self.on(LedColor::None);
        self.last_blinking_color = LedColor::None; // Reset to avoid any lag on blinking
    }
}

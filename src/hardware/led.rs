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
pub const LED_DYNMAC_GO_WAIT_KEY: u8 = 6;
pub const LED_DYNMAC_REC_WAIT_KEY: u8 = 7;

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
    neopixel: &'a mut Neopixel<'a>,
}

impl Led<'_> {
    pub fn new<'a>(neopixel: &'a mut Neopixel<'a>) -> Led<'a> {
        Led { neopixel }
    }

    pub fn light_on(&mut self, color: LedColor) {
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
                            }
                            .into(),
                        ),
                        30,
                    ))
                    .unwrap();
             }
        }
    }
    pub fn light_off(&mut self) {
        self.neopixel
            .write(brightness(once([0, 0, 0].into()), 3))
            .unwrap();
    }
}

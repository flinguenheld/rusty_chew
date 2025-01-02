use waveshare_rp2040_zero as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use defmt_rtt as _;
use embedded_hal::digital::*;
use panic_probe as _;

pub struct Gpios {
    pub pins: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
}

impl Gpios {
    pub fn update_states(&mut self) -> [u8; 4] {
        let mut pin_state_buffer = [0; 4];

        for (i, row) in self.pins.iter_mut().enumerate() {
            for k in row.iter_mut() {
                if let Some(key) = k {
                    pin_state_buffer[i] <<= 1;
                    if key.is_low().unwrap() {
                        pin_state_buffer[i] |= 1;
                    }
                }
            }
        }

        pin_state_buffer
    }
}

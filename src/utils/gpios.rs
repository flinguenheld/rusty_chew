use heapless::Vec;
use waveshare_rp2040_zero as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use embedded_hal::digital::*;
use panic_probe as _;

pub struct Gpios {
    pub pins: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
}

impl Gpios {
    pub fn update_states(&mut self) -> [u8; 4] {
        let mut pin_state_buffer = [0; 4];

        for (i, row) in self.pins.iter_mut().enumerate() {
            for key in row.iter_mut().flatten() {
                pin_state_buffer[i] <<= 1;
                if key.is_low().unwrap() {
                    pin_state_buffer[i] |= 1;
                }
            }
        }

        pin_state_buffer
    }

    // 00  01  02  03  04    |    05  06  07  08  09
    // 10  11  12  13  14    |    15  16  17  18  19
    // 20  21  22  23        |        24  25  26  27
    //         28  29  30    |    31  32  33

    pub fn get_active_indexes(&mut self) -> Vec<u8, 17> {
        let mut output = Vec::new();

        for (row, i) in self.pins.iter_mut().zip([5, 15, 24, 31].iter()) {
            for (c, pin) in row.iter_mut().enumerate() {
                if let Some(key) = pin {
                    if key.is_low().unwrap_or(false) {
                        output.push((i + c) as u8).ok();
                    }
                }
            }
        }

        output
    }
}

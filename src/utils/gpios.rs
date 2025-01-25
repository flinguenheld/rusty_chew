use heapless::Vec;
use waveshare_rp2040_zero as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use embedded_hal::digital::*;
use panic_probe as _;

pub struct Gpios {
    pub pins: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
}

impl Gpios {
    // 00  01  02  03  04    |    05  06  07  08  09
    // 10  11  12  13  14    |    15  16  17  18  19
    // 20  21  22  23        |        24  25  26  27
    //         28  29  30    |    31  32  33

    pub fn read(&mut self) -> [u8; 4] {
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

    pub fn get_left_indexes(&mut self) -> Vec<u8, 8> {
        self.get_indexes([0, 10, 20, 28])
    }
    pub fn get_right_indexes(&mut self) -> Vec<u8, 8> {
        self.get_indexes([5, 15, 24, 31])
    }

    fn get_indexes(&mut self, indexes: [u8; 4]) -> Vec<u8, 8> {
        let mut output = Vec::new();
        for (row, index) in self.pins.iter_mut().zip(indexes.iter()) {
            for (i, key) in row.iter_mut().flatten().enumerate() {
                if key.is_low().unwrap() {
                    output.push(*index + i as u8).ok();
                }
            }
        }
        output
    }
}

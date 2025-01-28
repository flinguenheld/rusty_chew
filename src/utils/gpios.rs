use heapless::Vec;
use waveshare_rp2040_zero as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use embedded_hal::digital::*;
use panic_probe as _;

/// Regroup in an array the pins which are used by the controller.
/// Chew keyboard is wired in 'direct pins' so one pin = one key.
///
/// Here are the indexes which are used by the matrix. Gpios struct is in
/// charge of converting an active pin status into a matrix index.
///
/// 00  01  02  03  04    |    05  06  07  08  09
/// 10  11  12  13  14    |    15  16  17  18  19
/// 20  21  22  23        |        24  25  26  27
///         28  29  30    |    31  32  33
pub struct Gpios {
    pub pins: [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
}

impl Gpios {
    pub fn get_left_indexes(&mut self) -> Vec<u8, 8> {
        self.get_active_indexes([0, 10, 20, 28])
    }
    pub fn get_right_indexes(&mut self) -> Vec<u8, 8> {
        self.get_active_indexes([5, 15, 24, 31])
    }

    fn get_active_indexes(&mut self, indexes: [u8; 4]) -> Vec<u8, 8> {
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

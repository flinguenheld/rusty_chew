use cortex_m::delay::Delay;
use heapless::Vec;
use waveshare_rp2040_zero::{
    self as bsp,
    hal::gpio::{PullDown, SioOutput},
};

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use embedded_hal::digital::*;
use panic_probe as _;

/// -------------------------------------------------------------------------------------
/// ---------------------------------------------------------------------------- SPLIT --

/// Regroup in an array the pins which are used by the controller.
/// Chew keyboard (split version) is wired in 'direct pins' so one pin = one key.
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

/// -------------------------------------------------------------------------------------
/// ----------------------------------------------------------------------------- MONO --

const CHEW_INDEXES: [[Option<u8>; 4]; 10] = [
    [Some(0), Some(10), Some(20), None],
    [Some(1), Some(11), Some(21), None],
    [Some(2), Some(12), Some(22), Some(28)],
    [Some(3), Some(13), Some(23), Some(29)],
    [Some(4), Some(14), None, Some(30)],
    // ------------------------------------
    [Some(5), Some(15), None, Some(31)],
    [Some(6), Some(16), Some(24), Some(32)],
    [Some(7), Some(17), Some(25), Some(33)],
    [Some(8), Some(18), Some(26), None],
    [Some(9), Some(19), Some(27), None],
];

/// Active columns one by one and check rows.
/// Chew Mono is wired like that:
///
///       C   C   C   C   C          C   C   C   C   C
///       0   1   2   3   4          5   6   7   8   9
///       |   |   |   |   |          |   |   |   |   |
/// R0 - 00  01  02  03  04    |    05  06  07  08  09
/// R1 - 10  11  12  13  14    |    15  16  17  18  19
/// R2 - 20  21  22  23        |        24  25  26  27
/// R3 -         28  29  30    |    31  32  33
///
pub struct GpiosMono {
    pub rows: [Pin<DynPinId, FunctionSio<SioInput>, PullDown>; 4],
    pub columns: [Pin<DynPinId, FunctionSio<SioOutput>, PullDown>; 10],
}

impl GpiosMono {
    pub fn get_active_indexes(&mut self, delay: &mut Delay) -> Vec<u8, 16> {
        let mut output = Vec::new();

        for (index_col, col) in self.columns.iter_mut().enumerate() {
            if col.set_high().is_ok() {
                delay.delay_us(1);
                for (index_row, r) in self.rows.iter_mut().enumerate() {
                    if let Some(matrix_index) = CHEW_INDEXES[index_col][index_row] {
                        if r.is_high().unwrap_or(false) {
                            output.push(matrix_index).ok();
                        }
                    }
                }

                col.set_low().ok();
            }
        }

        output
    }
}

use embedded_hal::digital::InputPin;
use waveshare_rp2040_zero::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};

use super::timer::ChewTimer;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum MatrixStatus {
    Pressed(u32),
    Done(u32),
    Held,
    Released,
    Free,
}

pub struct Matrix {
    pub grid: [MatrixStatus; 34],
}

// 00  01  02  03  04    |    05  06  07  08  09
// 10  11  12  13  14    |    15  16  17  18  19
// 20  21  22  23        |        24  25  26  27
//         28  29  30    |    31  32  33

impl Matrix {
    pub fn new() -> Self {
        Matrix {
            grid: [MatrixStatus::Free; 34],
        }
    }

    pub fn read_left(
        &mut self,
        rows: &mut [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
        chew_timer: &ChewTimer,
    ) {
        let indexes = [0, 10, 20, 28];
        for (row, index_start) in rows.iter_mut().zip(indexes.iter()) {
            for (i, k) in row.iter_mut().enumerate() {
                if let Some(key) = k {
                    self.up_case_status(index_start + i, key.is_low().unwrap_or(false), chew_timer);
                }
            }
        }
    }

    // 0b000010000 -> J
    // 0b000001000 -> M
    // 0b000000100 -> M
    // 0b000000010 -> Y
    // 0b000000001 -> W
    pub fn read_right(&mut self, right: &mut [u8; 4], chew_timer: &ChewTimer) {
        let indexes = [(5, 9), (15, 19), (24, 27), (31, 33)];
        for (row, index) in right.iter_mut().zip(indexes.iter()) {
            for i in (index.0..=index.1).rev() {
                self.up_case_status(i, *row & 1 == 1, chew_timer);
                *row >>= 1;
            }
        }
    }

    // Upgrade grid status from the hardware perspective
    fn up_case_status(&mut self, index: usize, is_low: bool, chew_timer: &ChewTimer) {
        if is_low {
            match self.grid[index] {
                MatrixStatus::Pressed(saved_ticks) | MatrixStatus::Done(saved_ticks) => {
                    if chew_timer.diff(saved_ticks) > 150 {
                        self.grid[index] = MatrixStatus::Held;
                    }
                }
                MatrixStatus::Free => {
                    self.grid[index] = MatrixStatus::Pressed(chew_timer.ticks);
                }
                _ => {}
            }
        } else {
            self.grid[index] = match self.grid[index] {
                MatrixStatus::Pressed(_) => MatrixStatus::Released,
                _ => MatrixStatus::Free,
            };
        }
    }
}

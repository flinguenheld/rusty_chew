use core::mem::swap;
use heapless::Vec;

/// The matrix struct allows Chew to be aware of key evolutions.
/// It consists of two arrays of 34 integers.
/// On each update, the current matrix is saved and each keys are updated
/// according to the gpios status.
/// The value saved is the press time in milliseconds.
///
/// 00  01  02  03  04    |    05  06  07  08  09
/// 10  11  12  13  14    |    15  16  17  18  19
/// 20  21  22  23        |        24  25  26  27
///         28  29  30    |    31  32  33

pub struct Matrix {
    pub cur: [u32; 34],
    pub prev: [u32; 34],

    last_ticks: u32,
}

impl Matrix {
    pub fn new(ticks: u32) -> Matrix {
        Matrix {
            cur: [0; 34],
            prev: [0; 34],
            last_ticks: ticks,
        }
    }

    pub fn update(&mut self, active_indexes: Vec<u8, 16>, ticks: u32) {
        swap(&mut self.cur, &mut self.prev);
        let diff = match self.last_ticks <= ticks {
            true => ticks - self.last_ticks,
            false => ticks + (u32::MAX - self.last_ticks),
        };

        for index in 0..self.cur.len() {
            match active_indexes.contains(&(index as u8)) {
                true => self.cur[index] = self.prev[index] + diff,
                false => self.cur[index] = 0,
            }
        }

        self.last_ticks = ticks;
    }
}

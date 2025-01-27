use core::mem::swap;

use heapless::Vec;

use super::options::TIMER_MAIN_LOOP;

// 00  01  02  03  04    |    05  06  07  08  09
// 10  11  12  13  14    |    15  16  17  18  19
// 20  21  22  23        |        24  25  26  27
//         28  29  30    |    31  32  33

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

    // REMOVE --------------------------------********************
    // REMOVE --------------------------------********************
    // REMOVE --------------------------------********************
    pub fn up(&mut self, left_rows: [u8; 4], right_rows: [u8; 4]) {
        swap(&mut self.cur, &mut self.prev);

        self.up_side(left_rows, [(0, 4), (10, 14), (20, 23), (28, 30)]);
        self.up_side(right_rows, [(5, 9), (15, 19), (24, 27), (31, 33)]);
    }

    pub fn up_side(&mut self, mut rows: [u8; 4], indexes: [(usize, usize); 4]) {
        for (row, index) in rows.iter_mut().zip(indexes.iter()) {
            for i in (index.0..=index.1).rev() {
                match *row & 1 == 1 {
                    true => self.cur[i] = self.prev[i] + TIMER_MAIN_LOOP,
                    false => self.cur[i] = 0,
                }
                *row >>= 1;
            }
        }
    }
}

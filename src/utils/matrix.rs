use core::mem::{self, swap};
use heapless::{FnvIndexMap, Vec};

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

// pub struct MCase {
//     index: usize,
//     ticks: u32,
// }

pub struct Matrix {
    pub cur: [u32; 34],
    pub prev: [u32; 34],

    current: Vec<usize, 16>,
    previous: Vec<usize, 16>,
}

impl Matrix {
    pub fn new() -> Matrix {
        Matrix {
            cur: [0; 34],
            prev: [0; 34],

            previous: Vec::new(),
            current: Vec::new(),
        }
    }

    pub fn update_new(&mut self, active_indexes: Vec<u8, 16>) {
        mem::swap(&mut self.previous, &mut self.current);
        self.current = active_indexes.iter().map(|&v| v as usize).collect();
    }

    pub fn freshly_pressed(&self) -> Vec<usize, 16> {
        self.current
            .iter()
            .filter(|index| !self.previous.contains(index))
            .copied()
            .collect()
    }
    pub fn freshly_released(&self) -> Vec<usize, 16> {
        self.previous
            .iter()
            .filter(|index| !self.current.contains(index))
            .copied()
            .collect()
    }

    pub fn is_active(&self, index: usize) -> bool {
        self.current.contains(&index)
    }

    pub fn is_matrix_active(&self) -> bool {
        self.current != self.previous
    }
}

use embedded_hal::digital::InputPin;
use waveshare_rp2040_zero::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};

pub struct Matrix {
    pub grid: [[u32; 34]; 4],
}

impl Matrix {
    pub fn new() -> Self {
        Matrix { grid: [[0; 34]; 4] }
    }

    pub fn read_left(
        &mut self,
        rows: &mut [[Option<Pin<DynPinId, FunctionSio<SioInput>, PullUp>>; 5]; 4],
        ticks: u32,
    ) {
        for (i, row) in rows.iter_mut().enumerate() {
            for (j, k) in row.iter_mut().enumerate() {
                if let Some(key) = k {
                    match key.is_low().unwrap() {
                        true => self.grid[i][j] = ticks,
                        false => self.grid[i][j] = 0,
                    }
                }
            }
        }
    }

    // 0b000010000 -> J
    // 0b000001000 -> M
    // 0b000000100 -> M
    // 0b000000010 -> Y
    // 0b000000001 -> W
    pub fn read_right(&mut self, right: &mut [u8; 4], ticks: u32) {
        for (r, row) in right.iter_mut().enumerate() {
            for c in (5..=9).rev() {
                match *row & 1 {
                    0 => self.grid[r][c] = 0,
                    _ => self.grid[r][c] = ticks,
                }
                *row >>= 1;
            }
        }
    }
}

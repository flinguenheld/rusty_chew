// Infinite ticks from 10
pub struct ChewTimer {
    pub ticks: u32,
}
impl ChewTimer {
    pub fn new() -> Self {
        ChewTimer { ticks: 10 }
    }
    pub fn add(&mut self) {
        self.ticks = match self.ticks {
            u32::MAX => 10,
            _ => self.ticks + 1,
        }
    }

    pub fn diff(&self, val: u32) -> u32 {
        if val >= 10 {
            match self.ticks < val {
                true => u32::MAX - val + 1 + self.ticks,
                false => self.ticks - val,
            }
        } else {
            0
        }
    }
}

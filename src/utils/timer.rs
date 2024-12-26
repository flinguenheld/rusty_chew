pub struct ChewTimer {
    pub ticks: u32,
}
impl ChewTimer {
    pub fn new() -> Self {
        ChewTimer { ticks: 0 }
    }
    pub fn add(&mut self) {
        self.ticks = match self.ticks {
            u32::MAX => 0,
            _ => self.ticks + 1,
        }
    }

    pub fn diff(&self, val: u32) -> u32 {
        match self.ticks < val {
            true => u32::MAX - val + 1 + self.ticks,
            false => self.ticks - val,
        }
    }
}

/// Due to layers modifiers have to be manage with their matrix status directly.
/// This struct keeps the state and the matrix index for each modifier.
pub struct Modifiers {
    pub alt: (bool, usize),
    pub alt_gr: (bool, usize),
    pub ctrl: (bool, usize),
    pub gui: (bool, usize),
    pub shift: (bool, usize),
}

impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            alt: (false, 0),
            alt_gr: (false, 0),
            ctrl: (false, 0),
            gui: (false, 0),
            shift: (false, 0),
        }
    }

    pub fn is_active(&self, index: usize) -> bool {
        (self.alt.0 && self.alt.1 == index)
            || (self.alt_gr.0 && self.alt_gr.1 == index)
            || (self.ctrl.0 && self.ctrl.1 == index)
            || (self.gui.0 && self.gui.1 == index)
            || (self.shift.0 && self.shift.1 == index)
    }

    pub fn deactivate_released(&mut self, matrix: &[u32; 34]) {
        self.alt.0 = self.alt.0 && matrix[self.alt.1] > 0;
        self.alt_gr.0 = self.alt_gr.0 && matrix[self.alt_gr.1] > 0;
        self.ctrl.0 = self.ctrl.0 && matrix[self.ctrl.1] > 0;
        self.gui.0 = self.gui.0 && matrix[self.gui.1] > 0;
        self.shift.0 = self.shift.0 && matrix[self.shift.1] > 0;
    }

    #[rustfmt::skip]
    pub fn nb_on(&self) -> usize {
        let mut nb = 0;
        if self.alt.0 { nb += 1 }
        if self.alt_gr.0 { nb += 1 }
        if self.ctrl.0 { nb += 1 }
        if self.gui.0 { nb += 1 }
        if self.shift.0 { nb += 1 }
        nb
    }
}

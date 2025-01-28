use super::options::HOLD_TIME;

/// Due to layers and homerow mods, modifiers have to be manage with their matrix
/// status directly.
/// This struct keeps the state and the matrix index for each modifier.
pub struct Modifiers {
    pub alt: (bool, usize),
    pub alt_gr: (bool, usize),
    pub ctrl: (bool, usize),
    pub gui: (bool, usize),
    pub shift: (bool, usize),
}

#[rustfmt::skip]
impl Modifiers {
    pub fn new() -> Modifiers {
        Modifiers {
            alt: (false, usize::MAX),
            alt_gr: (false, usize::MAX),
            ctrl: (false, usize::MAX),
            gui: (false, usize::MAX),
            shift: (false, usize::MAX),
        }
    }

    pub fn is_active(&self, index: usize) -> bool {
        (self.alt.0 && self.alt.1 == index)
            || (self.alt_gr.0 && self.alt_gr.1 == index)
            || (self.ctrl.0 && self.ctrl.1 == index)
            || (self.gui.0 && self.gui.1 == index)
            || (self.shift.0 && self.shift.1 == index)
    }

    pub fn update_states(&mut self, matrix: &[u32; 34])
    {
        self.alt    = up(self.alt, matrix);
        self.alt_gr = up(self.alt_gr, matrix);
        self.ctrl   = up(self.ctrl, matrix);
        self.gui    = up(self.gui, matrix);
        self.shift  = up(self.shift, matrix);
    }

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

fn up(mut md: (bool, usize), matrix: &[u32; 34]) -> (bool, usize) {
    if md.1 < usize::MAX {
        if matrix[md.1] == 0 {
            md.0 = false;
            md.1 = usize::MAX;
        } else if matrix[md.1] >= HOLD_TIME {
            md.0 = true;
        }
    }
    md
}

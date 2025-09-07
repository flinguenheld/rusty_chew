use crate::options::BUZZER_STARTUP_ACTIVATION;

#[derive(PartialEq, Eq)]
pub enum Status {
    On,
    SwitchOn,
    Off,
    SwitchOff,
}

pub struct Statuses {
    pub layout_fr: Status,
    pub layout_fn: Status,
    pub leader_key: Status,
    pub caplock: Status,
    pub dynmac_rec_waitkey: Status,
    pub dynmac_rec_inprogess: Status,
    pub dynmac_go_waitkey: Status,
    pub buzzer_activation: Status,
}

impl Statuses {
    pub fn new() -> Self {
        Statuses {
            layout_fr: Status::Off,
            layout_fn: Status::Off,
            leader_key: Status::Off,
            caplock: Status::Off,

            dynmac_rec_waitkey: Status::Off,
            dynmac_rec_inprogess: Status::Off,
            dynmac_go_waitkey: Status::Off,

            buzzer_activation: if BUZZER_STARTUP_ACTIVATION {
                Status::On
            } else {
                Status::Off
            },
        }
    }

    pub fn up(&mut self, who: &str, value: bool) {
        match who {
            "FR" => self.layout_fr = next(&self.layout_fr, value),
            "FN" => self.layout_fn = next(&self.layout_fn, value),
            "LEADER" => self.leader_key = next(&self.leader_key, value),
            "CAPLOCK" => self.caplock = next(&self.caplock, value),

            "DN_REC_WAIT" => self.dynmac_rec_waitkey = next(&self.dynmac_rec_waitkey, value),
            "DN_REC_PROG" => self.dynmac_rec_inprogess = next(&self.dynmac_rec_inprogess, value),
            "DN_GO_WAIT" => self.dynmac_go_waitkey = next(&self.dynmac_go_waitkey, value),

            "BUZZER" => self.buzzer_activation = next(&self.buzzer_activation, value),
            _ => {}
        }
    }
}

fn next(s: &Status, value: bool) -> Status {
    match (s, value) {
        (Status::On, true) => Status::On,
        (Status::On, false) => Status::SwitchOff,

        (Status::SwitchOn, true) => Status::On,
        (Status::SwitchOn, false) => Status::SwitchOff,

        (Status::Off, true) => Status::SwitchOn,
        (Status::Off, false) => Status::Off,

        (Status::SwitchOff, true) => Status::SwitchOn,
        (Status::SwitchOff, false) => Status::Off,
    }
}

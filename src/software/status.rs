use crate::hardware::{
    buzzer::Song,
    led::{Led, LedColor},
};
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

// --------------------------------------------------------------------------------------
// ----------------------------------------------------------------------- STATUS BITS --

// Bit of each status in the byte sent to the slave, the lowest bit set gives the LED
const BIT_LAYOUT_FR: u8 = 0;
const BIT_LAYOUT_FN: u8 = 1;
const BIT_LEADER_KEY: u8 = 2;
const BIT_CAPLOCK: u8 = 3;

const BIT_DYNMAC_GO_WAIT: u8 = 4;
const BIT_DYNMAC_REC: u8 = 5;
const BIT_DYNMAC_REC_WAIT: u8 = 6;

const BIT_BUZZER: u8 = 7;

/// Statuses -> one byte, a bit is set while its status is on (sent to the slave on split).
pub fn statuses_to_bits(statuses: &Statuses) -> u8 {
    [
        (BIT_LAYOUT_FR, &statuses.layout_fr),
        (BIT_LAYOUT_FN, &statuses.layout_fn),
        (BIT_LEADER_KEY, &statuses.leader_key),
        (BIT_CAPLOCK, &statuses.caplock),
        (BIT_DYNMAC_GO_WAIT, &statuses.dynmac_go_waitkey),
        (BIT_DYNMAC_REC, &statuses.dynmac_rec_inprogess),
        (BIT_DYNMAC_REC_WAIT, &statuses.dynmac_rec_waitkey),
        (BIT_BUZZER, &statuses.buzzer_activation),
    ]
    .iter()
    .filter(|(_, status)| matches!(status, Status::On | Status::SwitchOn))
    .fold(0, |bits, (bit, _)| bits | (1 << bit))
}

pub fn is_buzzer_active(bits: u8) -> bool {
    bits & (1 << BIT_BUZZER) != 0
}

/// LED of the lowest status bit set (the buzzer bit has no color).
pub fn show_status_led(led: &mut Led, bits: u8, ticks: u32) {
    // trailing_zeros gives 8 when no bit is set
    match (bits & !(1 << BIT_BUZZER)).trailing_zeros() as u8 {
        BIT_LAYOUT_FR => led.on(LedColor::Aqua),
        BIT_LAYOUT_FN => led.on(LedColor::Fushia),
        BIT_LEADER_KEY => led.on(LedColor::Blue),
        BIT_CAPLOCK => led.on(LedColor::Orange),

        BIT_DYNMAC_GO_WAIT => led.blink(LedColor::Olive, 800, ticks),
        BIT_DYNMAC_REC => led.blink(LedColor::Red, 600, ticks),
        BIT_DYNMAC_REC_WAIT => led.blink(LedColor::Purple, 800, ticks),

        _ => led.off(),
    }
}

/// Song for the changes between two status bytes (song, reverse), the same on mono and on
/// both halves of the split. One song at a time, the lowest bit first.
pub fn changed_status_song(last: u8, bits: u8) -> Option<(Song, bool)> {
    let changed = last ^ bits;

    (0..8)
        .filter(|bit| changed & (1 << bit) != 0)
        .find_map(|bit| status_song(bit, bits & (1 << bit) != 0))
}

/// Song of a status bit which changed, None for no song.
fn status_song(bit: u8, switched_on: bool) -> Option<(Song, bool)> {
    match (bit, switched_on) {
        (BIT_LAYOUT_FR, on) => Some((Song::StartupB, !on)),
        (BIT_LAYOUT_FN, on) => Some((Song::NotifC, !on)),
        (BIT_LEADER_KEY, true) => Some((Song::NotifA, false)),
        (BIT_CAPLOCK, on) => Some((Song::StartupG, !on)),

        (BIT_DYNMAC_GO_WAIT, true) => Some((Song::NotifF, false)),
        (BIT_DYNMAC_REC, true) => Some((Song::AlertD, false)),
        (BIT_DYNMAC_REC_WAIT, true) => Some((Song::AlertE, false)),

        (BIT_BUZZER, on) => Some((Song::StartupA, !on)),
        _ => None,
    }
}

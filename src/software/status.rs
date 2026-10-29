use crate::hardware::{
    buzzer::{Buzzer, Song},
    led::{Led, LedColor},
};

// --------------------------------------------------------------------------------------
// ----------------------------------------------------------------------- STATUS BITS --

/// One bit per status, sent to the slave on split (2 bytes of the STATUS mail).
pub type StatusBits = u16;

// Bit of each status, the lowest bit set gives the LED. Bits 8 to 15 are free.
pub const BIT_LAYOUT_FR: u8 = 0;
pub const BIT_LAYOUT_FN: u8 = 1;
pub const BIT_LEADER_KEY: u8 = 2;
pub const BIT_CAPLOCK: u8 = 3;

pub const BIT_DYNMAC_GO_WAIT: u8 = 4;
pub const BIT_DYNMAC_REC: u8 = 5;
pub const BIT_DYNMAC_REC_WAIT: u8 = 6;

pub const BIT_BUZZER: u8 = 7;

/// (bit, on) -> status bits, a bit is set while its status is on.
pub fn to_bits(statuses: &[(u8, bool)]) -> StatusBits {
    statuses
        .iter()
        .filter(|(_, on)| *on)
        .fold(0, |bits, (bit, _)| bits | (1 << bit))
}

pub fn is_buzzer_active(bits: StatusBits) -> bool {
    bits & (1 << BIT_BUZZER) != 0
}

/// LED of the lowest status bit set (the buzzer bit has no color).
pub fn show_status_led(led: &mut Led, bits: StatusBits, ticks: u32) {
    // trailing_zeros gives 16 when no bit is set
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

/// Song for the changes between two status bits (song, reverse), the same on mono and on
/// both halves of the split. One song at a time, the lowest bit first.
pub fn changed_status_song(last: StatusBits, bits: StatusBits) -> Option<(Song, bool)> {
    let changed = last ^ bits;

    (0..StatusBits::BITS as u8)
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

/// Songs for the status changes since the last state (none without a last state).
/// The buzzer is enabled before them and disabled after, so the buzzer toggle song can be heard.
pub fn play_status_songs(buzzer: &mut Buzzer, last: Option<StatusBits>, bits: StatusBits) {
    let buzzer_active = is_buzzer_active(bits);

    if buzzer_active {
        buzzer.set_active(true);
    }
    if let Some((song, reverse)) = last.and_then(|last| changed_status_song(last, bits)) {
        buzzer.add_song(song, reverse);
    }
    if !buzzer_active {
        buzzer.set_active(false);
    }
}

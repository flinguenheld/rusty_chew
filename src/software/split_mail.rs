use heapless::Vec;

use crate::hardware::{
    buzzer::Song,
    led::{Led, LedColor},
    uart::FRAME_VALUES,
};
use crate::software::status::{Status, Statuses};

/// Content of the mails exchanged by the two halves (the frame itself is in uart.rs).
///
///     KEYS    (slave -> master)  Mask of the slave's pressed keys (17 bits), on 3 bytes
///     STATUS  (master -> slave)  Status bits, 0, 0
///
/// Both are states sent every cycle, a lost frame is caught up by the next one.

// --------------------------------------------------------------------------------------
// ------------------------------------------------------------------------- KEYS MASK --

// Matrix indexes of each half, bit n of the mask is the key at position n
const LEFT_INDEXES: [u8; 17] = [
    0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 20, 21, 22, 23, 28, 29, 30,
];
const RIGHT_INDEXES: [u8; 17] = [
    5, 6, 7, 8, 9, 15, 16, 17, 18, 19, 24, 25, 26, 27, 31, 32, 33,
];

/// (own side, other side) indexes, used to convert the keys mask.
pub fn sides(is_left: bool) -> (&'static [u8; 17], &'static [u8; 17]) {
    if is_left {
        (&LEFT_INDEXES, &RIGHT_INDEXES)
    } else {
        (&RIGHT_INDEXES, &LEFT_INDEXES)
    }
}

/// Slave's pressed keys -> mask, bit n is the key at position n of its side array.
pub fn indexes_to_mask(indexes: &[u8], side: &[u8; 17]) -> [u8; FRAME_VALUES] {
    let mut mask = 0u32;
    for index in indexes {
        if let Some(bit) = side.iter().position(|i| i == index) {
            mask |= 1 << bit;
        }
    }
    [mask as u8, (mask >> 8) as u8, (mask >> 16) as u8]
}

/// Mask -> slave's pressed keys.
/// A bit set above the 17 keys can only come from a corrupted frame which passed the CRC.
pub fn mask_to_indexes(mask: &[u8; FRAME_VALUES], side: &[u8; 17]) -> Option<Vec<u8, 17>> {
    let mask = u32::from_le_bytes([mask[0], mask[1], mask[2], 0]);
    if mask >> side.len() != 0 {
        return None;
    }

    Some(
        side.iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, index)| *index)
            .collect(),
    )
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

/// Master's statuses -> one byte, a bit is set while its status is on.
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

/// Song played by the slave for the changes between two status bytes (song, reverse).
/// One song per frame, the lowest bit first (same priority as the master).
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

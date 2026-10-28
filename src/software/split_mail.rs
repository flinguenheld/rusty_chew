use heapless::Vec;

use crate::hardware::uart::FRAME_VALUES;

/// Content of the mails exchanged by the two halves (the frame itself is in uart.rs).
///
///     STATUS  (master -> slave)  Status bits, 0, 0              Request, every 5 ms
///     KEYS    (slave -> master)  Mask of the slave's pressed keys    Answer (17 bits, on 3 bytes)
///
/// Both are states sent every cycle, a lost frame is caught up by the next one.
/// The status bits are built in status.rs, they also drive the LED and the songs.

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

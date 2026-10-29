use heapless::Vec;

use usbd_serial::SerialPort;
use waveshare_rp2040_zero::hal::usb::UsbBus;

use crate::software::{serial_usb::serial_write_value, status::StatusBits};
use crate::{hardware::uart::FRAME_VALUES, options::LINK_TIMEOUT};

/// Content of the mails exchanged by the two halves (the frame itself is in uart.rs).
///
///     STATUS  (master -> slave)  Status bits (2 bytes), 0       Request, every 5 ms
///     KEYS    (slave -> master)  Mask of the slave's pressed keys    Answer (17 bits, on 3 bytes)
///
/// Both are states sent every cycle, a lost frame is caught up by the next one.
/// The status bits are built by Chew::status_bits (bits in status.rs), they also drive the
/// LED and the songs.

// --------------------------------------------------------------------------------------
// ------------------------------------------------------------------------ STATUS BITS --

/// Master's status bits -> STATUS mail, low byte first, the last byte is free.
pub fn status_to_mail(bits: StatusBits) -> [u8; FRAME_VALUES] {
    let [low, high] = bits.to_le_bytes();
    [low, high, 0]
}

/// STATUS mail -> status bits.
pub fn mail_to_status(values: &[u8; FRAME_VALUES]) -> StatusBits {
    StatusBits::from_le_bytes([values[0], values[1]])
}

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
// ------------------------------------------------------------------------------ LINK --

/// State of the link, the same on both halves: lost after LINK_TIMEOUT without a valid mail.
pub struct Link {
    // Tick of the last valid mail
    last_mail_ticks: u32,
    lost: bool,
}

impl Link {
    pub fn new() -> Self {
        Link {
            last_mail_ticks: 0,
            lost: false,
        }
    }

    /// A valid mail arrived, the link is (back) up.
    pub fn received(&mut self, ticks: u32) {
        self.last_mail_ticks = ticks;
        self.lost = false;
    }

    /// True once, at the moment LINK_TIMEOUT has passed without a valid mail.
    pub fn just_lost(&mut self, ticks: u32) -> bool {
        if !self.lost && ticks.wrapping_sub(self.last_mail_ticks) > LINK_TIMEOUT {
            self.lost = true;
            return true;
        }
        false
    }

    pub fn is_lost(&self) -> bool {
        self.lost
    }
}

/// Frames dropped by the master, printed on serial every TIMER_LINK_REPORT.
pub struct DroppedFrames {
    pub crc: u32,
    pub bad_masks: u32,
    no_answers: u32,
    // Whether the slave answered the last request
    waiting_answer: bool,
}

impl DroppedFrames {
    pub fn new() -> Self {
        DroppedFrames {
            crc: 0,
            bad_masks: 0,
            no_answers: 0,
            waiting_answer: false,
        }
    }

    /// A new request goes out, the last one without answer is counted.
    pub fn request_sent(&mut self) {
        if self.waiting_answer {
            self.no_answers += 1;
        }
        self.waiting_answer = true;
    }

    pub fn answer_received(&mut self) {
        self.waiting_answer = false;
    }

    /// Print the counters on serial and start again from 0.
    pub fn report(&mut self, serial: &mut SerialPort<'_, UsbBus>) {
        serial_write_value(serial, "Dropped frames / s - CRC:", self.crc, "");
        serial_write_value(serial, "| Bad masks:", self.bad_masks, "");
        serial_write_value(serial, "| No answer:", self.no_answers, "\r\n");
        self.crc = 0;
        self.bad_masks = 0;
        self.no_answers = 0;
    }
}

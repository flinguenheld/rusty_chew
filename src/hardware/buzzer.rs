use embedded_hal::pwm::SetDutyCycle;
use heapless::Deque;
use waveshare_rp2040_zero::{
    hal::pwm::{FreeRunning, Slice, SliceId},
    XOSC_CRYSTAL_FREQ,
};

const DUTY_MAX: u32 = 5000; // 24_292 from channel.get_max_duty()

pub const C: u16 = (XOSC_CRYSTAL_FREQ / 262) as u16;
pub const C_SHARP: u16 = (XOSC_CRYSTAL_FREQ / 277) as u16;
pub const D: u16 = (XOSC_CRYSTAL_FREQ / 294) as u16;
pub const E_FLAT: u16 = (XOSC_CRYSTAL_FREQ / 311) as u16;
pub const E: u16 = (XOSC_CRYSTAL_FREQ / 330) as u16;
pub const F: u16 = (XOSC_CRYSTAL_FREQ / 349) as u16;
pub const F_SHARP: u16 = (XOSC_CRYSTAL_FREQ / 370) as u16;
pub const G: u16 = (XOSC_CRYSTAL_FREQ / 392) as u16;
pub const A_FLAT: u16 = (XOSC_CRYSTAL_FREQ / 415) as u16;
pub const A: u16 = (XOSC_CRYSTAL_FREQ / 440) as u16;
pub const B_FLAT: u16 = (XOSC_CRYSTAL_FREQ / 466) as u16;
pub const B: u16 = (XOSC_CRYSTAL_FREQ / 494) as u16;
pub const SILENCE: u16 = 0;
pub const TIME: u32 = 100;

pub struct Note {
    frequency: u16,
    ticks_length: u32,
    ticks_start: Option<u32>,
    duty_percentage: u32,
}
impl Note {
    pub fn new(frequency: u16, ticks_length: u32, duty_percentage: u32) -> Note {
        Note {
            frequency,
            ticks_length,
            ticks_start: None,
            duty_percentage,
        }
    }
}

pub struct Buzzer {
    channel: char,
    current: Deque<Note, 200>,
}

impl Buzzer {
    pub fn new(channel: char) -> Self {
        Buzzer {
            channel,
            current: Deque::new(),
        }
    }

    pub fn add_song(&mut self, mut song: Deque<Note, 50>) {
        while let Some(note) = song.pop_back() {
            self.current.push_front(note).ok();
        }
    }

    pub fn sing<I: SliceId>(&mut self, ticks: u32, pwm: &mut Slice<I, FreeRunning>) {
        if let Some(note) = self.current.front_mut() {
            if note.ticks_start.is_none() {
                note.ticks_start = Some(ticks);
            }

            if note.ticks_start.unwrap() + note.ticks_length >= ticks {
                // Sing
                pwm.set_top(note.frequency);
                match self.channel {
                    'A' => {
                        pwm.channel_a
                            .set_duty_cycle((DUTY_MAX * note.duty_percentage / 100) as u16)
                            .ok();
                    }
                    _ => {
                        pwm.channel_b
                            .set_duty_cycle((DUTY_MAX * note.duty_percentage / 100) as u16)
                            .ok();
                    }
                }
            } else {
                self.current.pop_front();
            }
        } else {
            pwm.channel_b.set_duty_cycle(SILENCE).ok();
        }
    }
}

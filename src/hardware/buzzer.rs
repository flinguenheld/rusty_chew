use embedded_hal::pwm::SetDutyCycle;
use heapless::Deque;
use waveshare_rp2040_zero::{
    hal::pwm::{FreeRunning, Slice, SliceId},
    XOSC_CRYSTAL_FREQ,
};

use crate::options::SONG_MAX_LENGTH;

const C4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 262.0) as u16;
const C4_SHARP: u16 = (XOSC_CRYSTAL_FREQ as f32 / 277.0) as u16;
const D4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 294.0) as u16;
const E4_FLAT: u16 = (XOSC_CRYSTAL_FREQ as f32 / 311.0) as u16;
const E4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 330.0) as u16;
const F4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 349.0) as u16;
const F4_SHARP: u16 = (XOSC_CRYSTAL_FREQ as f32 / 370.0) as u16;
const G4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 392.0) as u16;
const A4_FLAT: u16 = (XOSC_CRYSTAL_FREQ as f32 / 415.0) as u16;
const A4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 440.0) as u16;
const B4_FLAT: u16 = (XOSC_CRYSTAL_FREQ as f32 / 466.0) as u16;
const B4: u16 = (XOSC_CRYSTAL_FREQ as f32 / 494.0) as u16;
const TIME: u32 = 200;

pub enum Song {
    JingleBells,
    EMinor_Up,
    EMinor_down,
}

pub struct Note {
    frequency: u16,
    ticks_length: u32,
    ticks_start: Option<u32>,
}
impl Note {
    pub fn new(frequency: u16, ticks_length: u32) -> Note {
        Note {
            frequency,
            ticks_length,
            ticks_start: None,
        }
    }
}

pub struct Buzzer<I: SliceId> {
    pwm: Slice<I, FreeRunning>,
    score: Deque<Note, SONG_MAX_LENGTH>,
}

impl<I: SliceId> Buzzer<I> {
    pub fn new(mut pwm: Slice<I, FreeRunning>) -> Self {
        // Compromise to get low range notes
        pwm.set_div_int(8);
        pwm.set_div_frac(8);
        pwm.enable();

        Buzzer {
            pwm,
            score: Deque::new(),
        }
    }

    #[rustfmt::skip]
    pub fn add_song(&mut self, song: Song) {
        match song {
            Song::JingleBells => {
                self.score.push_back(Note::new(E4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(E4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(E4, TIME * 2)).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();

                self.score.push_back(Note::new(E4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(E4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(E4, TIME * 2)).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();

                self.score.push_back(Note::new(E4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(G4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(C4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
                self.score.push_back(Note::new(D4, TIME    )).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();

                self.score.push_back(Note::new(E4, TIME * 4)).ok();
                self.score.push_back(Note::new(0 , TIME / 4)).ok();
            }

            Song::EMinor_Up=>{
                        self.score.push_back(Note::new(C4,      TIME / 4)).ok();
                        self.score.push_back(Note::new(E4_FLAT, TIME / 4)).ok();
                        self.score.push_back(Note::new(G4,      TIME / 4)).ok();
                        self.score.push_back(Note::new(0,       TIME / 4)).ok();
            },
            Song::EMinor_down=>{
                        self.score.push_back(Note::new(G4,      TIME / 4)).ok();
                        self.score.push_back(Note::new(E4_FLAT, TIME / 4)).ok();
                        self.score.push_back(Note::new(C4,      TIME / 4)).ok();
                        self.score.push_back(Note::new(0,       TIME / 4)).ok();
            },

            
        }
    }

    pub fn sing(&mut self, ticks: u32) {
        if let Some(note) = self.score.front_mut() {
            if note.ticks_start.is_none() {
                note.ticks_start = Some(ticks);
            }

            if note.ticks_start.unwrap() + note.ticks_length >= ticks {
                self.pwm.set_top(note.frequency);
                self.pwm.channel_a.set_duty_cycle_percent(50).ok();
                self.pwm.channel_b.set_duty_cycle_percent(50).ok();
            } else {
                self.score.pop_front();
            }
        } else {
            self.pwm.set_top(0);
            self.pwm.channel_b.set_duty_cycle(0).ok();
        }
    }
}

use embedded_hal::pwm::SetDutyCycle;
use heapless::Deque;
use waveshare_rp2040_zero::{
    hal::pwm::{FreeRunning, Slice, SliceId},
    XOSC_CRYSTAL_FREQ,
};

use crate::options::{BUZZER_ON, BUZZER_STARTUP_ACTIVATION, SONG_MAX_LENGTH};

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
const TIME: u32 = 100;

#[allow(dead_code)]
pub enum Song {
    JingleBells,

    WelcomeC,

    TwoCE,
    TwoFA,
    TwoGB,
    TwoAC,
    TwoDF,

    StartupC,
    Startup2C,
    StartupF,
    StartupG,
    StartupA,
    StartupB,

    NotifC,
    NotifF,
    NotifG,
    NotifA,

    ErrorE,
    ErrorF,
    ErrorG,

    SuccessA,
    SuccessB,
    SuccessC,

    AlertD,
    AlertE,
    AlertF,
}

pub enum Side {
    Left,
    Right,
    Both,
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
    sheet_music: Deque<Note, SONG_MAX_LENGTH>,
    active: bool,
}

impl<I: SliceId> Buzzer<I> {
    pub fn new(mut pwm: Slice<I, FreeRunning>) -> Self {
        // Compromise to get low range notes
        pwm.set_div_int(8);
        pwm.set_div_frac(8);
        pwm.enable();

        Buzzer {
            pwm,
            sheet_music: Deque::new(),
            active: BUZZER_STARTUP_ACTIVATION,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn sing(&mut self, ticks: u32) {
        if let Some(note) = self.sheet_music.front_mut() {
            if note.ticks_start.is_none() {
                note.ticks_start = Some(ticks);
            }

            if note.ticks_start.unwrap() + note.ticks_length >= ticks {
                self.pwm.set_top(note.frequency);
                self.pwm.channel_a.set_duty_cycle_percent(50).ok();
                self.pwm.channel_b.set_duty_cycle_percent(50).ok();
            } else {
                self.sheet_music.pop_front();
            }
        } else {
            self.pwm.set_top(0);
            self.pwm.channel_b.set_duty_cycle(0).ok();
        }
    }

    /// Add notes in the sheet music according to their side and the given side.
    /// Side adapt the sheet music by replacing notes by silences if needed.
    /// Reverse true to reverse the song but keep the order of the times.
    fn add(
        &mut self,
        sheet_music: Deque<(Note, Side), SONG_MAX_LENGTH>,
        side: &Side,
        reverse: bool,
    ) {
        let mut sheet_music = if reverse {
            let mut temp = Deque::new();

            for (a, b) in sheet_music.iter().zip(sheet_music.iter().rev()) {
                temp.push_back((
                    Note {
                        frequency: b.0.frequency,
                        ticks_length: a.0.ticks_length,
                        ticks_start: None,
                    },
                    match b.1 {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                        Side::Both => Side::Both,
                    },
                ))
                .ok();
            }

            temp
        } else {
            sheet_music
        };

        while let Some(note) = sheet_music.pop_front() {
            match (side, note.1) {
                (Side::Left, Side::Left) | (Side::Right, Side::Right) => {
                    self.sheet_music.push_back(note.0).ok();
                }
                (Side::Right, Side::Left) | (Side::Left, Side::Right) => {
                    self.sheet_music
                        .push_back(Note {
                            frequency: 0,
                            ..note.0
                        })
                        .ok();
                }
                _ => {
                    self.sheet_music.push_back(note.0).ok();
                }
            }
        }
    }

    #[rustfmt::skip]
    /// Add a song in the buzzer sheet music.
    /// Side allow you to say which side is playing. So notes which are define in the other side,
    /// will be replaced by silences.
    /// Reverse to invert the notes and the sides but keep the order times.
    pub fn add_song(&mut self, song: Song, side: Side, reverse: bool) {
        if BUZZER_ON && self.active {
            let mut new_sheet_music: Deque<(Note, Side), SONG_MAX_LENGTH> = Deque::new();
            match song {
                Song::JingleBells => {
                    new_sheet_music .push_back((Note::new(E4          , TIME     ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME     ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME * 2 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Left  )).ok();

                    new_sheet_music .push_back((Note::new(E4          , TIME     ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME     ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME * 2 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();

                    new_sheet_music .push_back((Note::new(E4          , TIME     ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(G4          , TIME     ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(C4          , TIME     ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(D4          , TIME     ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();

                    new_sheet_music .push_back((Note::new(E4          , TIME * 4 ), Side::Both  )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Both  )).ok();
                }

                Song::TwoCE => {
                    new_sheet_music .push_back((Note::new(C4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME / 4 ), Side::Right )).ok();
                }
                Song::TwoFA => {
                    new_sheet_music .push_back((Note::new(F4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(A4          , TIME / 4 ), Side::Right )).ok();
                }
                Song::TwoGB => {
                    new_sheet_music .push_back((Note::new(G4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(B4          , TIME / 4 ), Side::Right )).ok();
                }
                Song::TwoAC => {
                    new_sheet_music .push_back((Note::new(A4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(C4 / 2      , TIME / 4 ), Side::Right )).ok();
                }
                Song::TwoDF => {
                    new_sheet_music .push_back((Note::new(D4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(F4_SHARP    , TIME / 4 ), Side::Right )).ok();
                }
                
                Song::WelcomeC => {
                    new_sheet_music .push_back((Note::new(C4          , TIME / 2 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME / 2 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(G4          , TIME / 2 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(C4 / 2      , TIME     ), Side::Both  )).ok();
                }
                
                Song::StartupC => {
                    new_sheet_music .push_back((Note::new(C4          , TIME / 3 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4          , TIME / 6 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(G4          , TIME / 3 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(C4 / 2      , TIME / 6 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(E4 / 2      , TIME / 2 ), Side::Both  )).ok();
                }

                Song::Startup2C => {
                    new_sheet_music .push_back((Note::new(C4          , TIME / 5 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(E4_FLAT     , TIME / 5 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(G4          , TIME / 5 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(B4          , TIME / 3 ), Side::Right )).ok();
                }
                Song::StartupF => {
                    new_sheet_music .push_back((Note::new(F4          , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(A4          , TIME / 2 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(C4 / 2      , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(F4 / 2      , TIME / 2 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(A4 / 2      , TIME     ), Side::Both  )).ok();
                }
                Song::StartupG => {
                    new_sheet_music .push_back((Note::new(G4          , TIME / 6 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(B4          , TIME / 6 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(D4 / 2      , TIME / 4 ), Side::Both  )).ok();
                }
                Song::StartupA => {
                    new_sheet_music .push_back((Note::new(A4          , TIME     ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(C4_SHARP / 2, TIME / 2 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(E4 / 2      , TIME / 3 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(A4 / 2      , TIME / 4 ), Side::Both  )).ok();
                }
                Song::StartupB => {
                    new_sheet_music .push_back((Note::new(B4_FLAT     , TIME / 5 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(D4 / 2      , TIME / 5 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(F4 / 2      , TIME / 3 ), Side::Both  )).ok();
                }

                Song::NotifC => {
                    new_sheet_music .push_back((Note::new(C4          , TIME / 7 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(C4          , TIME / 7 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(0           , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(D4          , TIME / 5 ), Side::Both  )).ok();
                }
                Song::NotifF => {
                    new_sheet_music .push_back((Note::new(F4 / 2      , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(A4 / 2      , TIME / 4 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(C4          , TIME / 4 ), Side::Right )).ok();
                }
                Song::NotifG => {
                    new_sheet_music .push_back((Note::new(G4          , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(B4          , TIME / 2 ), Side::Left  )).ok();
                    new_sheet_music .push_back((Note::new(D4 / 2      , TIME / 4 ), Side::Right )).ok();
                    new_sheet_music .push_back((Note::new(G4 / 2      , TIME / 2 ), Side::Left  )).ok();
                }
                Song::NotifA => {
                    new_sheet_music .push_back((Note::new(A4           , TIME / 4 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(C4_SHARP / 2 , TIME / 4 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(E4 / 2       , TIME / 4 ), Side::Right)).ok();
                }

                Song::ErrorE => {
                    new_sheet_music .push_back((Note::new(E4_FLAT      , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(E4           , TIME / 2 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(E4_FLAT      , TIME / 2 ), Side::Left )).ok();
                }
                Song::ErrorF => {
                    new_sheet_music .push_back((Note::new(F4           , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(0            , TIME / 5 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(F4_SHARP     , TIME / 2 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(0            , TIME / 5 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(F4           , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(0            , TIME / 5 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(F4_SHARP     , TIME / 2 ), Side::Both )).ok();
                }
                Song::ErrorG => {
                    new_sheet_music .push_back((Note::new(G4           , TIME / 3 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(A4_FLAT      , TIME / 3 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(G4           , TIME     ), Side::Right)).ok();
                }

                Song::SuccessA => {
                    new_sheet_music .push_back((Note::new(A4_FLAT      , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(C4 / 2       , TIME / 2 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(E4_FLAT / 2  , TIME     ), Side::Left )).ok();
                }
                Song::SuccessB => {
                    new_sheet_music .push_back((Note::new(B4_FLAT      , TIME / 3 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(D4 / 2       , TIME / 3 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(F4 / 2       , TIME / 3 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(B4_FLAT / 2  , TIME / 2 ), Side::Left )).ok();
                }
                Song::SuccessC => {
                    new_sheet_music .push_back((Note::new(C4           , TIME / 3 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(E4           , TIME / 3 ), Side::Both )).ok();
                    new_sheet_music .push_back((Note::new(G4           , TIME     ), Side::Right)).ok();
                }

                Song::AlertD => {
                    new_sheet_music .push_back((Note::new(C4_SHARP     , TIME / 3 ), Side::Both )).ok();
                    new_sheet_music .push_back((Note::new(F4           , TIME / 3 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(C4_SHARP     , TIME / 2 ), Side::Left )).ok();
                }
                Song::AlertE => {
                    new_sheet_music .push_back((Note::new(E4_FLAT      , TIME / 2 ), Side::Both )).ok();
                    new_sheet_music .push_back((Note::new(G4           , TIME / 2 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(E4_FLAT / 2  , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(G4 / 2       , TIME     ), Side::Right)).ok();
                }
                Song::AlertF => {
                    new_sheet_music .push_back((Note::new(F4           , TIME / 2 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(0            , TIME / 5 ), Side::Left )).ok();
                    new_sheet_music .push_back((Note::new(A4           , TIME / 2 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(0            , TIME / 5 ), Side::Right)).ok();
                    new_sheet_music .push_back((Note::new(F4           , TIME / 2 ), Side::Both )).ok();
                }
            }

            self.add(new_sheet_music, &side, reverse);
        }
    }
}

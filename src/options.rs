use cfg_if::cfg_if;

pub const HOLD_TIME: u32 = 150; // From free to held (ms)
pub const COMBO_TIME: u32 = 20;
pub const SONG_MAX_LENGTH: usize = 50;

pub const BUFFER_LENGTH: usize = 100;
pub const BUFFER_CASE_LENGTH: usize = 10;

pub const NB_KEYS: usize = 34;
pub const TEMPO_DEAD_KEY: u32 = 50;

// Milliseconds
pub const TIMER_MONO_LOOP: u32 = 5;
pub const TIMER_USB_LOOP: u32 = 15;
pub const TIMER_UART_LOOP: u32 = 1;
pub const TIMER_SPLIT_LOOP: u32 = 5; // New slave index sending

pub const UART_SPEED: u32 = 921_600;
// pub const UART_SPEED: u32 = 460_800;
// pub const UART_SPEED: u32 = 115_200;
// pub const UART_SPEED: u32 = 19_200;
pub const MAX_MESSAGE_LENGTH: usize = 9; // Max tested in January 2025
pub const MAX_NOT_COMPLETE: u32 = 3;
pub const UART_SEND_DELAY: u32 = 500; // microseconds

// Mouse
// Move i8 each u32 ms
pub const MOUSE_SPEED_1: (i8, u32) = (1, 15);
pub const MOUSE_SPEED_2: (i8, u32) = (2, 10);
pub const MOUSE_SPEED_3: (i8, u32) = (5, 10);
pub const MOUSE_SPEED_4: (i8, u32) = (20, 10);
pub const MOUSE_SPEED_DEFAULT: (i8, u32) = (10, 10);

pub const SCROLL_SPEED_1: (i8, u32) = (1, 120);
pub const SCROLL_SPEED_2: (i8, u32) = (1, 80);
pub const SCROLL_SPEED_3: (i8, u32) = (5, 10);
pub const SCROLL_SPEED_4: (i8, u32) = (15, 10);
pub const SCROLL_SPEED_DEFAULT: (i8, u32) = (1, 20);

cfg_if! {
    if #[cfg(feature = "serial")] {
        pub const SERIAL_ON: bool = true;
    } else {
        pub const SERIAL_ON: bool = false;
    }
}

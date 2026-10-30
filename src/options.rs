use cfg_if::cfg_if;

pub const HOLD_TIME: u64 = 150; // From free to held (ms)
pub const COMBO_TIME: u64 = 20;

pub const BUZZER_ON: bool = true;
pub const BUZZER_STARTUP_ACTIVATION: bool = true;
pub const SONG_MAX_LENGTH: usize = 50;

pub const BUFFER_LENGTH: usize = 100;
pub const BUFFER_CASE_LENGTH: usize = 16;
pub const NKRO_MAX_KEYS: usize = 11; // Held together, + 5 mods <= BUFFER_CASE_LENGTH

pub const NB_KEYS: usize = 34;

// Milliseconds
pub const TIMER_MONO_LOOP: u32 = 5;
pub const TIMER_USB_LOOP: u64 = 10; // Minimum gap between two reports (>= host poll), a lone key goes out at once
pub const TIMER_UART_LOOP: u32 = 1;
pub const TIMER_SPLIT_LOOP: u32 = 5; // Master asks the slave for its keys
pub const TIMER_LINK_REPORT: u32 = 1000; // Master prints the dropped frames on serial
pub const LINK_TIMEOUT: u64 = 100; // Without a valid mail, the link is lost (keys released)

// pub const UART_SPEED: u32 = 921_600;
// pub const UART_SPEED: u32 = 460_800;
pub const UART_SPEED: u32 = 115_200;
// pub const UART_SPEED: u32 = 19_200;
pub const MAX_MESSAGE_LENGTH: usize = 9; // RX FIFO (8 bytes) + shift register, a frame is 5 bytes
pub const MAX_NOT_COMPLETE: u32 = 3;
pub const UART_SEND_DELAY: u32 = 100; // microseconds, guard before a frame (other side's stop bit)

pub const ROWS_DISCHARGE_MAX: u32 = 30; // microseconds, mono: max wait for the rows to go low between two columns

// Mouse
// Move i8 each u64 ms
pub const MOUSE_SPEED_1: (i8, u64) = (1, 15);
pub const MOUSE_SPEED_2: (i8, u64) = (2, 10);
pub const MOUSE_SPEED_3: (i8, u64) = (5, 10);
pub const MOUSE_SPEED_4: (i8, u64) = (20, 10);
pub const MOUSE_SPEED_DEFAULT: (i8, u64) = (10, 10);

pub const SCROLL_SPEED_1: (i8, u64) = (1, 120);
pub const SCROLL_SPEED_2: (i8, u64) = (1, 80);
pub const SCROLL_SPEED_3: (i8, u64) = (5, 10);
pub const SCROLL_SPEED_4: (i8, u64) = (15, 10);
pub const SCROLL_SPEED_DEFAULT: (i8, u64) = (1, 20);

cfg_if! {
    if #[cfg(feature = "serial")] {
        pub const SERIAL_ON: bool = true;
    } else {
        pub const SERIAL_ON: bool = false;
    }
}

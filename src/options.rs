pub const HOLD_TIME: u32 = 170; // From free to held (ms)
pub const COMBO_TIME: u32 = 20;

pub const BUFFER_LENGTH: usize = 50;
pub const BUFFER_CASE_LENGTH: usize = 10;

pub const NB_KEYS: usize = 34;
pub const TEMPO_DEAD_KEY: u32 = 50;

// Milliseconds
pub const TIMER_UART_LOOP: u32 = 1;
pub const TIMER_MONO_LOOP: u32 = 5;
pub const TIMER_USB_LOOP: u32 = 15;

pub const UART_SPEED: u32 = 115_200;
// pub const UART_SPEED: u32 = 19200;
pub const UART_SEND_DELAY: u32 = 100; // microseconds

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

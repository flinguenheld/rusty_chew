pub const HOLD_TIME: u32 = 170; // From free to held (ms)
pub const COMBO_TIME: u32 = 20;

pub const BUFFER_LENGTH: usize = 50;
pub const BUFFER_CASE_LENGTH: usize = 10;

pub const TEMPO_DEAD_KEY: u32 = 50;

// Milliseconds
pub const TIMER_UART_LOOP: u32 = 1;
pub const TIMER_USB_LOOP: u32 = 15;

// pub const UART_SPEED: u32 = 115_200;
pub const UART_SPEED: u32 = 19200;
pub const UART_SEND_DELAY: u32 = 500; // microseconds

// Mouse
pub const MOUSE_SPEED_1: i8 = 1;
pub const MOUSE_SPEED_2: i8 = 4;
pub const MOUSE_SPEED_3: i8 = 25;
pub const MOUSE_SPEED_4: i8 = 35;
pub const MOUSE_SPEED_DEFAULT: i8 = 15;

pub const SCROLL_SPEED_1: (u32, i8) = (20, 1);
pub const SCROLL_SPEED_2: (u32, i8) = (10, 1);
pub const SCROLL_SPEED_3: (u32, i8) = (1, 8);
pub const SCROLL_SPEED_4: (u32, i8) = (1, 15);
pub const SCROLL_SPEED_DEFAULT: (u32, i8) = (2, 1);

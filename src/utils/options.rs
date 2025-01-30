pub const HOLD_TIME: u32 = 150; // From free to held (ms)
pub const BUFFER_LENGTH: usize = 20;

// Milliseconds
pub const TIMER_UART_LOOP: u32 = 5;
pub const TIMER_USB_LOOP: u32 = 25;
pub const TIMER_LED_STARTUP: u32 = 2_000;

// pub const UART_SPEED: u32 = 115_200;
pub const UART_SPEED: u32 = 19200;
pub const UART_SEND_DELAY: u32 = 500; // microseconds

// Mouse
pub const MOUSE_SPEED_1: i8 = 1;
pub const MOUSE_SPEED_2: i8 = 5;
pub const MOUSE_SPEED_3: i8 = 22;
pub const MOUSE_SPEED_4: i8 = 40;
pub const MOUSE_SPEED_DEFAULT: i8 = 12;

pub const SCROLL_TEMP_SPEED_1: (u32, i8) = (80, 1);
pub const SCROLL_TEMP_SPEED_2: (u32, i8) = (40, 1);
pub const SCROLL_TEMP_SPEED_3: (u32, i8) = (10, 3);
pub const SCROLL_TEMP_SPEED_4: (u32, i8) = (0, 10);
pub const SCROLL_TEMP_SPEED_DEFAULT: (u32, i8) = (15, 1);

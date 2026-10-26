use waveshare_rp2040_zero as bsp;

use bsp::{
    hal::{
        gpio::bank0::Gpio11,
        gpio::{FunctionPio0, Pin, PullUp},
        pio::{
            PIOBuilder, PinDir, PinState, Running, Rx, ShiftDirection, StateMachine, Tx,
            UninitStateMachine, PIO, SM1, SM2,
        },
    },
    pac::PIO0,
};
use cortex_m::delay::Delay;
use fugit::HertzU32;

/// Half-duplex UART (8N1) on a single pin, with two PIO state machines:
///     - SM1 receives all the time, the pin stays an input
///     - SM2 transmits, it takes the pin only for the bits of each byte and releases it
///       after the stop bit (the pull-up keeps the line high between bytes)
///
/// The level is set before the pin becomes an output, so switching to transmit can't make
/// a low pulse on the line (pio-uart set the direction only).
/// Programs inspired by the pico-sdk uart_rx/uart_tx and the rktk half-duplex split driver.
///
/// The receiver also hears what is sent, this echo is removed at the end of `write`.
pub struct HalfDuplex {
    rx: Rx<(PIO0, SM1)>,
    tx: Tx<(PIO0, SM2)>,

    // Kept to stay running and to own the pin
    _rx_sm: StateMachine<(PIO0, SM1), Running>,
    _tx_sm: StateMachine<(PIO0, SM2), Running>,
    _pin: Pin<Gpio11, FunctionPio0, PullUp>,

    // Time of one byte on the wire (start + 8 bits + stop) plus one bit of margin
    byte_time_us: u32,
}

impl HalfDuplex {
    pub fn new(
        pio: &mut PIO<PIO0>,
        sm1: UninitStateMachine<(PIO0, SM1)>,
        sm2: UninitStateMachine<(PIO0, SM2)>,
        pin: Pin<Gpio11, FunctionPio0, PullUp>,
        baud: u32,
        sys_freq: HertzU32,
    ) -> HalfDuplex {
        let pin_id = pin.id().num;

        // Both programs run at 8 PIO cycles per bit
        let (div_int, div_frac) = clock_divisor(sys_freq.to_Hz(), baud);

        // Receive --
        let rx_program = pio_proc::pio_asm!(
            ".wrap_target",
            "    wait 0 pin 0",  // Start bit
            "    set x, 7 [10]", // Middle of the first data bit (1.5 bit)
            "bitloop:",
            "    in pins, 1",          // Sample a data bit
            "    jmp x-- bitloop [6]", // 8 cycles per bit
            "    wait 1 pin 0",        // Stop bit, the line must be high before a new start bit
            ".wrap"
        );
        let rx_installed = pio.install(&rx_program.program).unwrap();
        let (mut rx_sm, rx, _) = PIOBuilder::from_installed_program(rx_installed)
            .in_pin_base(pin_id)
            .in_shift_direction(ShiftDirection::Right)
            .autopush(true)
            .push_threshold(8)
            .buffers(bsp::hal::pio::Buffers::OnlyRx)
            .clock_divisor_fixed_point(div_int, div_frac)
            .build(sm1);
        rx_sm.set_pindirs([(pin_id, PinDir::Input)]);

        // Transmit --
        let tx_program = pio_proc::pio_asm!(
            ".wrap_target",
            "    pull", // Wait for a byte, the pin is released (input)
            "    set x, 7",
            "    set pins, 0",        // Low level ready before taking the pin
            "    set pindirs, 1 [7]", // Take the pin: start bit
            "bitloop:",
            "    out pins, 1",         // Data bit (LSB first)
            "    jmp x-- bitloop [6]", // 8 cycles per bit
            "    set pins, 1 [7]",     // Stop bit
            "    set pindirs, 0",      // Release the pin
            ".wrap"
        );
        let tx_installed = pio.install(&tx_program.program).unwrap();
        let (mut tx_sm, _, tx) = PIOBuilder::from_installed_program(tx_installed)
            .set_pins(pin_id, 1)
            .out_pins(pin_id, 1)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(false)
            .buffers(bsp::hal::pio::Buffers::OnlyTx)
            .clock_divisor_fixed_point(div_int, div_frac)
            .build(sm2);
        tx_sm.set_pins([(pin_id, PinState::High)]);
        tx_sm.set_pindirs([(pin_id, PinDir::Input)]);

        HalfDuplex {
            rx,
            tx,
            _rx_sm: rx_sm.start(),
            _tx_sm: tx_sm.start(),
            _pin: pin,
            byte_time_us: 11 * 1_000_000_u32.div_ceil(baud),
        }
    }

    /// Send the bytes and return once they are on the wire (blocking).
    /// The bytes must fit in the TX FIFO (8 bytes).
    pub fn write(&mut self, bytes: &[u8], delay: &mut Delay) {
        for b in bytes {
            while !self.tx.write(*b as u32) {}
        }

        // The FIFO is empty when the last byte starts, then wait for that byte
        while !self.tx.is_empty() {}
        delay.delay_us(self.byte_time_us);

        // The receiver heard the frame too
        while self.rx.read().is_some() {}
    }

    /// One received byte, None when the FIFO is empty.
    pub fn read(&mut self) -> Option<u8> {
        // Shifted to the right, the byte is in the 8 upper bits
        self.rx.read().map(|v| (v >> 24) as u8)
    }
}

/// System clock / (8 cycles × baud) as integer and 1/256 parts.
fn clock_divisor(sys_hz: u32, baud: u32) -> (u16, u8) {
    let cycles_per_s = 8 * baud as u64;
    let div_256 = (sys_hz as u64 * 256) / cycles_per_s;
    ((div_256 >> 8) as u16, (div_256 & 0xFF) as u8)
}

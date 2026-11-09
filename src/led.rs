use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::{ws2812, ws2812::PioWs2812};
use embassy_time::{Duration, Instant, Ticker, Timer};
use smart_leds::RGB8;

use crate::options::LED_FADE_FRAME;

#[cfg(feature = "zero")]
pub type Ws2812 = PioWs2812<'static, PIO0, 0, 1, ws2812::Grb>;
#[cfg(not(feature = "zero"))]
pub type Ws2812 = PioWs2812<'static, PIO0, 0, 1, ws2812::Rgb>;

// ──────────                                    ─────────────────────────────
// ─────                                   ────────────────────── COLORS ─────
pub const RED: RGB8 = RGB8::new(255, 0, 0);
pub const GREEN: RGB8 = RGB8::new(0, 255, 0);
pub const BLUE: RGB8 = RGB8::new(0, 0, 255);
pub const OFF: RGB8 = RGB8::new(0, 0, 0);

pub struct Led {
    ws: Ws2812,
    color: RGB8,
    new_color: RGB8,
}

impl Led {
    // ──────────                                ─────────────────────────────
    // ─────                               ───────────────────────── NEW ─────
    /// Wrap the WS2812 driver. Both colors start at `OFF`,
    /// and nothing is sent until the first `go` or `fade`.
    pub fn new(ws: Ws2812) -> Self {
        Led { ws, color: OFF, new_color: OFF }
    }

    // ──────────                                ─────────────────────────────
    // ─────                               ───────────────────────── SET ─────
    /// Store a new color. Nothing is shown until `go`, `fade`...
    pub fn set(&mut self, color: RGB8, dim: u8) -> &mut Self {
        self.new_color = Led::dim_color(color, dim);
        self
    }

    /// Store OFF as new color. Nothing is shown until `go`, `fade`...
    pub fn off(&mut self) -> &mut Self {
        self.new_color = OFF;
        self
    }

    // ──────────                                ─────────────────────────────
    // ─────                                ──────────────── GO / GO FOR ─────
    /// Activate the new color.
    pub async fn go(&mut self) -> &mut Self {
        self.color = self.new_color;
        self.write().await;
        self
    }

    /// Activate the new color and wait for the end of time_ms.
    pub async fn go_for(&mut self, time_ms: u64) -> &mut Self {
        self.go().await;
        Timer::after(Duration::from_millis(time_ms)).await;
        self
    }

    // ──────────                                ─────────────────────────────
    // ─────                               ──────────────── FADE / FLASH ─────
    /// Go from the current color to the new one in `duration_ms`.
    ///
    /// Every `LED_FADE_FRAME`, the elapsed time is turned into a progress:
    /// - 0: the current color,
    /// - 256: the new color.
    ///
    /// The fade always ends exactly on the new color,
    /// and the LED is only written when the color changes.
    ///
    /// If the new color is already the current one, it only waits for
    /// `duration_ms`: a sequence keeps the same length.
    pub async fn fade(&mut self, duration_ms: u64) -> &mut Self {
        /// Give the current value according to the progress.
        const fn blend_channel(from: u8, to: u8, progress: u32) -> u8 {
            let from_part = from as u32 * (256 - progress);
            let to_part = to as u32 * progress;
            ((from_part + to_part) >> 8) as u8
        }

        let ticks = Duration::from_millis(duration_ms).as_ticks();
        if ticks > 0 {
            let start = Instant::now();
            let mut ticker = Ticker::every(LED_FADE_FRAME);
            let (from, to) = (self.color, self.new_color);

            let mut t = 0;
            while t < ticks {
                ticker.next().await;

                // Progress never goes past 256
                // (no underflow in `256 - progress`).
                // And the last frame is exactly `to`.
                t = start.elapsed().as_ticks().min(ticks);
                let progress = (t * 256 / ticks) as u32;

                let color = RGB8::new(
                    blend_channel(from.r, to.r, progress),
                    blend_channel(from.g, to.g, progress),
                    blend_channel(from.b, to.b, progress),
                );

                if color != self.color {
                    self.color = color;
                    self.write().await;
                }
            }
        } else {
            self.go().await;
        }
        self
    }

    /// Fade to the new color, then fade back to off.
    /// Lasts 2 × `duration_ms`.
    pub async fn flash(&mut self, duration_ms: u64) -> &mut Self {
        self.fade(duration_ms).await.off().fade(duration_ms).await;
        self
    }

    // ──────────                                ─────────────────────────────
    // ─────                               ─────────────────────── WRITE ─────
    /// Send the current color to the LED, gamma corrected.
    ///
    /// The LED is linear: a value of 128 gives half the light of 255.
    /// The eye is not: it sees small changes in dim light much more than
    /// in bright light, so 128 looks much brighter than half.
    ///
    /// `gamma` (exponent 2.0) lowers the values before sending,
    /// so that equal steps in `color` look like equal steps in brightness.
    /// `sent = 255 × (c / 255)²`, computed in integers as `c² / 255`:
    ///
    /// ```text
    /// color    0   11   12   64   128   192   255
    /// sent     0    0    1   16    64   145   255
    /// ```
    ///
    /// `color` itself stays linear, so `fade` and `dim_color` work with
    /// plain values. Only the value sent to the LED is corrected.
    ///
    /// Limit: 0 to 11 are all sent as 0, so the LED is off for them.
    async fn write(&mut self) {
        // + 127 (half of 255): rounds to the nearest value.
        let gamma = |c: u8| (((c as u32).pow(2) + 127) / 255) as u8;
        let sent = RGB8::new(
            gamma(self.color.r),
            gamma(self.color.g),
            gamma(self.color.b),
        );
        self.ws.write(&[sent]).await;
    }

    // ──────────                                ─────────────────────────────
    // ─────                               ───────────────────────── DIM ─────
    /// Scale each channel by `dim`: 0 (off) to 255 (full color), rounded
    /// down.
    ///
    /// Linear, before gamma: once sent, a channel below 12 is off (see
    /// `write`). E.g. `RED` dimmed by 11, or `(128, 0, 0)` dimmed by 20
    /// (gives 10), both leave the LED off.
    pub const fn dim_color(mut rgb: RGB8, dim: u8) -> RGB8 {
        rgb.r = (rgb.r as u16 * dim as u16 / 255) as u8;
        rgb.g = (rgb.g as u16 * dim as u16 / 255) as u8;
        rgb.b = (rgb.b as u16 * dim as u16 / 255) as u8;
        rgb
    }
}

<div align="center">
  <img src="../images/crab_coffee.png">
</div>

The RGB LED of the keyboard: one WS2812 on GP16, driven by the PIO with DMA.
The code is in `src/led.rs`, the setup in `src/mono.rs`.

The interrupts it needs (`PIO0_IRQ_0`, `DMA_IRQ_0`) are bound for the whole
firmware, see [INTERRUPTS.md](./INTERRUPTS.md). PIO, DMA, FIFO:
[RP2040.md](./RP2040.md).

### Hardware

|             |                                                                      |
| ----------- | -------------------------------------------------------------------- |
| LED         | one WS2812 (a "NeoPixel": the LED and its controller in one package) |
| Pin         | GP16                                                                 |
| Driver      | `PioWs2812` (embassy-rp): PIO0, state machine 0, DMA channel 0       |
| Color order | `Grb` with the `zero` feature, `Rgb` otherwise (`Ws2812` type)       |

A WS2812 receives 24 bits (8 per channel) on one wire, then latches them after a
pause. The PIO makes the waveform, the DMA feeds it the color, so the CPU is
free during the transfer.

```
       24 bits × 1.25 µs                 reset (line low)
  ├──────────── ~30 µs ────────────┤├─────── 55 µs ───────┤
                                                          ▲ color shown

  one write ≈ 85 µs  (fade frame: 5 ms, so under 2 % of it)
```

<div align="center">
  <img src="../images/prawns.png">
</div>

### Two Colors

`Led` keeps two colors:

- `new_color`: the color asked for. `set` and `off` only store it.
- `color`: the color shown, linear (before gamma). `go` and `fade` move it
  toward `new_color` and write it.

```
set(color, dim) ─┐
                 ├─► new_color ──► go / go_for ─┐
off() ───────────┘    (stored)     fade / flash ┴─► color ──► gamma ──► LED
                                                   (linear)    (sent)
```

So a call always reads as "what" then "how", and the calls chain:

```rust
led.set(led::RED, 200)       // what: red, dimmed to 200 / 255
    .fade(300)               // how: in 300 ms
    .await
    .off()
    .fade(1000)
    .await;
```

| Method            | Does                                                    |
| ----------------- | ------------------------------------------------------- |
| `set(color, dim)` | store `color` scaled by `dim` (0 off, 255 full)         |
| `off()`           | store `OFF`                                             |
| `go()`            | show the new color at once                              |
| `go_for(ms)`      | show it, then wait `ms`                                 |
| `fade(ms)`        | go from the current color to the new one in `ms`        |
| `flash(ms)`       | fade to the new color, then fade to off: lasts 2 × `ms` |

<div align="center">
  <img src="../images/prawns.png">
</div>

### Fade

#### Time Based

Every `LED_FADE_FRAME` (5 ms, 200 Hz, in `src/options.rs`) the ticker wakes the
loop, which reads the time since the start and turns it into a **progress** from
0 (current color) to 256 (new color). The color only depends on the time, not on
how many frames ran: a late frame catches up by itself, and the fade always
lasts `ms`.

```
fade(500):

t (ms)     0     5    10    15   ...   495   500
frame            │     │     │          │     │
progress   0     2     5     7   ...   253   256  ◄─ clamped
color      from  ─────────────────────────►    to
```

```rust
t = start.elapsed().as_ticks().min(ticks);
let progress = (t * 256 / ticks) as u32;
```

- **`.min(ticks)`**: the last wake-up is almost always a bit late (502 ms fade,
  frames at 500 and 505 ms). Clamping keeps progress at 256 at most: no
  underflow in `256 - progress`, and the last frame is exactly the new color. It
  also ends the loop (`while t < ticks`).
- **In `u64`**: ticks are µs (1 MHz), so `t × 256` would overflow a `u32` after
  ~16.7 s.
- **`ticks == 0`** (`fade(0)`): `go` at once, so no division by zero.

#### Blend

Each channel is a weighted average of its two ends:

```
value = (from × (256 − progress) + to × progress) >> 8
```

| progress | part of `from` | part of `to` | value        |
| -------- | -------------- | ------------ | ------------ |
| 0        | 256 / 256      | 0            | `from`       |
| 64       | 192 / 256      | 64 / 256     | ¼ of the way |
| 128      | 128 / 256      | 128 / 256    | halfway      |
| 256      | 0              | 256 / 256    | `to`         |

- **256, not 255**: `>> 8` divides by 256 in one instruction (the M0+ has no
  divide), and at 256 the result is exactly `to`.
- **`u32`**: the sum is at most 255 × 256 = 65280. Registers are 32 bits, so
  `u32` costs nothing more than a smaller type.
- **Same progress for r, g, b**: the channels keep their ratio, so the hue stays
  the same. Orange fading to off stays orange, just darker.

#### Writes

The LED is only written when the blended color changes. On a slow fade, several
frames in a row can give the same color, so the end is decided by the time,
never by the color.

<div align="center">
  <img src="../images/prawns.png">
</div>

### Gamma

#### Why

The LED is linear: 128 gives half the light of 255 (PWM: on half the time). The
eye is not: it sees small changes in dim light much more than in bright light.
Without correction, 128 looks like 70–80 %, and a fade from off rushes at the
start then seems to stall at the top.

Gamma correction lowers the values before sending them, much more for small
values than for big ones, so equal steps in `color` look like equal steps in
brightness. A coefficient can't do that: it scales everything by the same ratio.
It needs a curve.

#### The Curve

`write` uses an exponent of 2.0:

```
sent = 255 × (color / 255)²  =  color² / 255
```

```
sent
255 ┤                                 ●
    │                             ·
192 ┤                         ·   ●
    │                     ·   ●
128 ┤                 ·
    │             ·       ●
 64 ┤         ·       ●
    │     ·   ●   ●            · linear (no gamma)
  0 ┤ ●   ●                    ● gamma 2.0
    └─┬───────┬───────┬───────┬───────┬
      0      64     128     192     255  color
```

| color | 0 | 11 | 12 | 64 | 128 | 192 | 255 |
| ----- | - | -- | -- | -- | --- | --- | --- |
| sent  | 0 | 0  | 1  | 16 | 64  | 145 | 255 |

```rust
// + 127 (half of 255): rounds to the nearest value.
let gamma = |c: u8| (((c as u32).pow(2) + 127) / 255) as u8;
```

- **`+ 127`**: integer division rounds down. Adding half the divisor first
  rounds to the nearest value instead (12² / 255 = 0.56 gives 1, not 0).
- **`.pow(2)`**: integer `pow`, from `core`. With 2 it compiles to one multiply,
  like `c * c`.
- **A formula, not a table**: with 2.0 it is one multiply and one divide by a
  constant (turned into a multiply and a shift). Another exponent (2.2, 2.8)
  would need `powf`: not in `core`, and slow without an FPU. That is why LED
  libraries use a precomputed table.

`color` itself stays linear: `fade` and `dim_color` work with plain values, only
the value sent is corrected.

#### The Dark End

The LED has 8 bits: nothing between off and 1 / 255. The curve is so flat at the
bottom that the first values give less than half of that step, so they round
to 0.

|                                        | off for |
| -------------------------------------- | ------- |
| gamma 2.0, rounded (ours)              | 0 to 11 |
| gamma 2.0, rounded down                | 0 to 15 |
| gamma 2.8 (smart-leds, Adafruit table) | 0 to 27 |

smart-leds' `gamma` uses the Adafruit table, exponent 2.8, chosen by eye for LED
strips showing animations: there the middle tones matter, and the dark end is
barely visible anyway. For one indicator LED with dim levels and fades from off,
the dark end matters more, hence the softer 2.0.

<div align="center">
  <img src="../images/prawns.png">
</div>

### Points to Keep in Mind

- **Low dim leaves the LED off**: after gamma, a channel below 12 is off. With a
  pure color (`RED`, `GREEN`, `BLUE`), `dim` 1 to 11 gives nothing. With a
  dimmer base color the limit is higher: `(128, 0, 0)` dimmed by 20 gives 10,
  off.
- **Fades from or to off**: the LED stays dark for the first (or last) ~5 % of
  the fade (12 / 256), while the channels are below 12.
- **Visible steps at the dark end**: 1 → 2 → 3 doubles then adds half the light.
  It is the 8-bit limit of the WS2812; only dithering could hide it.
- **Fade to the same color**: nothing is written, but it still waits `ms`, so a
  sequence keeps the same length.
- **Same output, extra writes**: `fade` compares linear colors, and a few linear
  values give the same sent value (e.g. 12 to 19 all give 1). On a slow, dark
  fade a few writes send what the LED already shows. Harmless (~85 µs each).

<div align="center">
  <img src="../images/crab_back.png">
</div>

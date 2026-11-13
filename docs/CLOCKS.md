<div align="center">
  <img src="./images/crab_coffee.png">
</div>

Where the RP2040 gets its sense of time: a quartz crystal on the board, and
the PLLs that turn its 12 MHz into the clocks of the CPU, the USB, the UART…

All of it is set by `embassy_rp::init`, with
`ClockConfig::crystal(12_000_000)` (the default `Config`, in
`embassy-rp-0.10.0/src/lib.rs`). The chip's vocabulary:
[RP2040.md](./RP2040.md).

### The Crystal

The RP2040-Zero has a **12 MHz quartz crystal** next to the chip: on the
board, not in the chip.

Quartz is **piezoelectric**: bent, it makes a small voltage; given a voltage,
it bends. A thin slice cut to a precise size vibrates at one exact frequency,
like a tuning fork. A small amplifier in the chip (the **XOSC**, crystal
oscillator) gives it a push on every cycle, and the quartz sets the pace:

```text
       ┌────── amplifier (XOSC, in the chip) ──────┐
       │                                           │
       └──────────── [ quartz ] <──────────────────┘
                     on the board
                         │
                         └──> 12 MHz, very stable
```

#### Two Oscillators

|          | XOSC                     | ROSC                                    |
| -------- | ------------------------ | --------------------------------------- |
| What     | amplifier + the crystal  | ring oscillator: a loop of gates inside |
| Where    | crystal on the board     | all in the chip                         |
| Accuracy | tens of ppm              | changes with voltage and temperature    |
| Used     | everything, after `init` | at reset, until `init` switches         |

The chip boots on the ROSC (it needs nothing outside), then `init` starts the
XOSC, waits for it to settle, and moves every clock to it.

```text
 power on ──> ROSC ticks ──> the CPU runs the boot code
                               └─> init: start the XOSC, wait, switch
                                         └─> everything on the crystal
```

#### The Ring Oscillator

The ROSC is an odd number of **NOT gates** in a loop. The value that comes
back is always the opposite of the one that left, so the loop never settles:
every wire of it carries a square wave. The chip takes it off one wire and
uses it as its clock, as is (nothing is measured).

```text
      ┌──> NOT ──> NOT ──> NOT ──┬──> clock out: ┌┐┌┐┌┐┌┐┌┐
      │                          │
      └──────────────────────────┘

 A=1 → B=0 → C=1 → A=0 → B=1 → C=0 → A=1 → …
```

- **Odd**: with 2 or 4 gates, the value that comes back is the same, and the
  loop stops.
- **Its speed** is only the delay of the gates (a few ns each). Fewer gates
  (`RoscRange::High` in embassy-rp) make it faster.

The same trick as the relay wired back to itself in Petzold's *Code*, with
transistors.

#### Why Not the ROSC All the Time

The delay of a gate depends on **PVT**: the **P**rocess (no two chips are the
same), the **V**oltage and the **T**emperature. So the ROSC moves:

```text
           1.8                6.5            12 MHz
 ROSC:      ├──────────────────●──────────────┤    ← moves with PVT
 XOSC:                                        ●    ← 12 MHz ± tens of ppm
```

| Part              | Needs              | With the ROSC                        |
| ----------------- | ------------------ | ------------------------------------ |
| USB               | 48 MHz ±0.25 %     | ✗ the PC rejects the device          |
| UART (split link) | ≈ ±5 % in total    | ⚠ maybe, on a good day               |
| Timer             | the real time      | ⚠ drifts by minutes per day          |
| LED (WS2812)      | ns, with a margin  | ⚠ probably OK                        |

The USB settles it: `ClockConfig::rosc()` sets `usb_clk: None`, no USB at
all. The ROSC is enough for a board without a crystal and without USB, or for
a sleeping chip (it uses less power than the XOSC).

<div align="center">
  <img src="./images/prawns.png">
</div>

### The Clock Tree

One crystal, several clocks. The values set by `ClockConfig::crystal`:

```text
                       ┌─> PLL_SYS  125 MHz ──┬─> clk_sys  125 MHz  CPU, PIO, DMA
                       │                      └─> clk_peri 125 MHz  UART, SPI, I2C
 crystal ──> XOSC ─────┼─> PLL_USB   48 MHz ──┬─> clk_usb   48 MHz  USB
 12 MHz      12 MHz    │                      ├─> clk_adc   48 MHz  ADC
                       │                      └─> clk_rtc   46875 Hz (÷1024)  RTC
                       └──────────────────────> clk_ref   12 MHz ──> tick 1 MHz  Timer
```

| Clock      | Frequency | From       | Used by in the Chew                          |
| ---------- | --------- | ---------- | -------------------------------------------- |
| `clk_sys`  | 125 MHz   | `PLL_SYS`  | the CPU, the PIO (WS2812 timings, [LED.md](./LED.md)), the DMA |
| `clk_peri` | 125 MHz   | `clk_sys`  | the UARTs ([UART.md](./UART.md))             |
| `clk_usb`  | 48 MHz    | `PLL_USB`  | the USB controller ([USB.md](./USB.md))      |
| `clk_ref`  | 12 MHz    | XOSC       | the 1 MHz tick of the timer: `embassy-time`  |
| `clk_adc`  | 48 MHz    | `PLL_USB`  | not used                                     |
| `clk_rtc`  | 46875 Hz  | `PLL_USB`  | not used                                     |

The timer counts **µs**: `init` sets the watchdog tick to divide `clk_ref`
by 12 (`clocks.rs`, `WATCHDOG.tick()`). That's the time base of
`Timer::after_millis`.

#### The PLLs

A **PLL** (Phase-Locked Loop) multiplies a clock. Inside, a fast oscillator
(the **VCO**, 750 to 1600 MHz) is locked on the crystal × `fbdiv`, then
divided down twice:

```text
 out = 12 MHz ÷ refdiv × fbdiv ÷ post_div1 ÷ post_div2

 PLL_SYS:  12 ÷ 1 × 125 = 1500 MHz (VCO) ÷ 6 ÷ 2 = 125 MHz
 PLL_USB:  12 ÷ 1 × 120 = 1440 MHz (VCO) ÷ 6 ÷ 5 =  48 MHz
```

The outputs are as precise as the crystal: the PLL follows it, it doesn't
add its own drift. Two PLLs because the USB needs exactly 48 MHz, and 125 is
not a multiple of 48.

<div align="center">
  <img src="./images/prawns.png">
</div>

### Precision

The error of a clock is given in **ppm** (parts per million): 1 ppm = 1 µs
too fast or too slow per second.

| Who       | Allowed error         | With the crystal (tens of ppm)   |
| --------- | --------------------- | -------------------------------- |
| USB       | ±0.25 % (2500 ppm)    | far below: the PC accepts it     |
| UART      | about ±5 % in total   | far below ([UART.md](./UART.md)) |
| Timer     | none: it's the time   | a few seconds a day at most      |

#### The Divider Trap

A clock can be precise and the result still be off: a peripheral **divides**
its clock, and the division is not always exact. The UART divides `clk_peri`
by `16 × baud`, with 6 bits of fraction (steps of 1/64):

```text
 115200 baud:  125 MHz ÷ (16 × 115200) = 67.817…
 closest step: 67 + 52/64              = 67.8125
 real baud:    125 MHz ÷ (16 × 67.8125) = 115 207    error 0.006 %
```

Here it's tiny. On chips with an integer-only divider, some baud rates came
out a few % off: the start of the 5 % margin.

<div align="center">
  <img src="./images/prawns.png">
</div>

### Aside: Why Watches Use 32768 Hz

Not in the Chew, but the same idea. A quartz watch has a **32768 Hz**
crystal, and the number is chosen for three reasons.

**A power of 2** (2¹⁵): a flip-flop divides by 2, so 15 of them in a chain
give exactly 1 tick per second. No counter to reset, just a chain.

```text
 32768 ──> 16384 ──> 8192 ──> … ──> 4 ──> 2 ──> 1 Hz
        ÷2        ÷2               ÷2    ÷2
       └──────────────── 15 flip-flops ───────────────┘
```

**Not too fast**: each tick switches the flip-flops and costs a little
energy. At 32 kHz a button cell lasts years, at 12 MHz a few days.

**Not too slow**:

| Power of 2 | Hz        | Problem                                     |
| ---------- | --------- | ------------------------------------------- |
| 2¹³        | 8192      | audible: the quartz could whine             |
| 2¹⁴        | 16384     | still audible for kids and young adults     |
| **2¹⁵**    | **32768** | above human hearing (about 20 kHz) ✓        |
| 2¹⁶        | 65536     | works, but twice the power for nothing      |

A slower quartz is also bigger (a longer tuning fork): at 32 kHz it's a tiny
fork a few mm long.

So 32768 is **the lowest power of 2 that can't be heard**. It's in watches,
and in the clock chips that keep the time in PCs and phones when they're off.

<div align="center">
  <img src="./images/crab_back.png">
</div>

<div align="center">
    <img src="../images/crab_coffee.png">
</div>

When things happen, from the press to the computer, and what each timer costs.

### Timers

All in milliseconds, counted with `ticks` (+1 every 1 ms) in `src/split.rs`
and `src/mono.rs`. Values in `src/options.rs`.

| Timer | Value | Role |
|---|---|---|
| `TIMER_MONO_LOOP` | 5 | Mono: reads the matrix and runs the chew logic |
| `TIMER_SPLIT_LOOP` | 5 | Split: the master sends a request, the chew logic runs on the answer |
| `TIMER_UART_LOOP` | 1 | Split: both sides read their UART buffer |
| `TIMER_USB_LOOP` | 10 | Minimum gap between two reports, not below the host poll |
| Host poll | 10 | The computer reads a report (HID crate default) |
| `COMBO_TIME` | 20 | A key waits in `pre_pressed_keys`, a combo can catch it |
| `HOLD_TIME` | 150 | A home-row key held this long becomes a modifier |
| `UART_SEND_DELAY` | 0.1 | Guard before a frame (the other side's stop bit) |
| `LINK_TIMEOUT` | 100 | Without a valid mail, the link is lost |
| `TIMER_LINK_REPORT` | 1000 | The master prints the dropped frames on serial |
| `MOUSE_SPEED_*`, `SCROLL_SPEED_*` | 10-120 | One mouse or wheel step at most every … |

The master's timers all run on the same hardware timer, so they never drift
from each other. The slave has its own crystal: its 1 ms reads slowly slide
against the master's.

<div align="center">
    <img src="../images/prawns.png">
</div>

### One split cycle

Every 5 ms the master asks, the slave answers, and the chew logic runs on the
answer. There's no chew timer on split: `update_matrix` + `run` only happen
when a valid KEYS mail is read. One possible cycle (one character = 0.1 ms,
`░` = `UART_SEND_DELAY`):

```
ms            0         1         2         3         4         5
              |·········|·········|·········|·········|·········|
master sends  ░▓▓▓▓  STATUS
slave                   ▲ reads it (1 ms tick)
slave sends             ░▓▓▓▓  KEYS
master                            ▲ reads it → chew run → report written
                                                                ▼ next request
```

| Step | At 115200 |
|---|---|
| Master sends (guard + 5 bytes) | 0.53 ms |
| Slave reads its buffer | 0-1 ms |
| Slave answers | 0.53 ms |
| Master reads its buffer | 0-1 ms |
| **Total** | **1.1-3.1 ms** |

- Both halves are read at nearly the same moment: the slave's keys about
  0.5-1.5 ms before the master's.
- A lost answer means no run for that cycle: the next one is 10 ms later.
- The two 1 ms reads weigh more than the wire: at 921600 the total would still
  be 0.3-2.3 ms.
- `send()` blocks the loop while the frame goes out, about 0.5 ms every 5 ms
  at 115200.

On mono, the chew logic runs on the `TIMER_MONO_LOOP` tick itself, with no
exchange.

### The 5 ms samples

The matrix is read every 5 ms (mono tick or split cycle), so `ticks` jump by 5
between two updates. The thresholds keep their real duration, rounded up to
the next sample:

```
 sample           1     2     3     4     5        one every 5 ms
 first key ticks  1     6     11    16    21 > 20 → pressed_keys
 second key       ✔     ✔     ✔     ✔     ✘        too late for a combo
```

- A key resolves 20-25 ms after its press: up to 5 ms before it's seen, then
  `COMBO_TIME`.
- **The combo window is 4 samples:** the second key must be seen while the
  first is at 16 at most. That's 15 ms between samples, so 10 to 20 ms between
  the two presses, depending on where they fall between samples. A combo that
  sometimes doesn't trigger can come from there.
- Only multiples of 5 change something: `COMBO_TIME` = 22 behaves like 25.
- Keys first seen in the same sample lose their real order: they come in the
  order of the matrix (on split, the master's keys first, then the slave's).
- `HOLD_TIME` (150 ms) moves by 5 ms at most, too little to feel.
- The sampling also acts as a debounce: switch bounce is usually shorter than
  5 ms.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Minimum gap

There's no USB clock: a report is written as soon as it's allowed, at least
`TIMER_USB_LOOP` (10 ms) after the last one. `a` then `b` pressed quickly, one
character = 0.5 ms:

```
ms            0         5         10        15        20        25        30
              |·········|·········|·········|·········|·········|·········|
split request ▼         ▼         ▼         ▼         ▼         ▼         ▼
chew run          ●         ●         ●         ●         ●         ●
pushed            [A]       [A, B]
written           ▲───────────────────▲
                  [A]  10 ms gap      [A, B]
```

- `[A]` goes out in the same loop pass as its run: the queue was empty and the
  gap long past.
- `[A, B]` waits for the 10 ms gap, on a 1 ms tick, not for the next run.

In normal typing the reports are far apart (a keystroke takes ~200 ms at 60
wpm), so most presses and releases go out at once. The gap only spaces out
the dense moments: sequences, macros, fast rolls.

Before, the USB loop ran on its own 15 ms clock: a lone key waited for the
next slot, 3, 8 or 13 ms on split (always the same three, since 15 = 3 × 5).

**Why 10 ms:** the computer reads a report every 10 ms (host poll). A smaller
gap gains nothing: a report written before the last one was read gets
`WouldBlock`, waits in the queue and is tried again at the next loop pass.
The 15 ms used before probably dated from the dead keys, when the loop itself
had to be slow so that the computer read them (see
[REPORTS.md](./REPORTS.md#example-shift-held-then-ê)).

### Latency

From the press to the computer, + `HOLD_TIME` for home-row keys:

| Stage | Cost |
|---|---|
| Matrix sample | 0-5 ms |
| Split exchange (split only) | ~1-3 ms |
| `COMBO_TIME` | 20 ms |
| USB gap | 0 ms alone, up to 10 ms behind another report |
| Host poll | 0-10 ms |
| **Total** | **about 21-38 ms** for a lone key |

- **Host poll:** the default `NKROBootKeyboardConfig` of the HID crate sets
  the IN endpoint to 10 ms (`in_endpoint(10.millis())`). It can be set with a
  custom config instead of `default()`.
- **Throughput:** one report per 10 ms at most is 100 reports/s.
  `MacroNixOS` (≈ 45 reports) takes about 0.45 s, and keys typed meanwhile
  wait behind it. NKRO pushes one report per change (press, release,
  modifier), so rolls with several keys held can queue up a little.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Possible tests

By gain for the effort, one at a time:

1. **Gap at the real host poll:** the keyboard asks for 10 ms, but most hosts
   round a full-speed interval down to a power of two and read every 8 ms.
   To measure it, set `TIMER_USB_LOOP` to 1 for a moment (the reports then go
   out as fast as the computer reads them), fire a long macro and look at the
   spacing of the events in `sudo evtest`. A gap of 8 would make sequences and
   macros 20% faster on such a host. On a host that really reads every 10 ms
   (another computer, a hub, a KVM), 8 only adds `WouldBlock` retries, never
   losses, and makes the red LED flicker during macros on mono.
2. **Host poll at 1 ms** (custom HID config): up to 10 ms less on every
   report, and then a much smaller gap becomes possible.
3. **`COMBO_TIME`:** 20 ms added to every key. Lowering it is a feel test: do
   the combos still trigger reliably? Keep a multiple of the sample period.
4. **The split period:** 4 ms fits the round trip and gains ~0.5 ms on
   average. Lower needs reading the UART on every loop pass (not on the 1 ms
   tick) and a faster `UART_SPEED`, see [SPLIT_LINK.md](./SPLIT_LINK.md#serial-debug)
   for the counters to watch. Keep one chew run per exchange: running the
   chew logic faster than the exchange would read the master's keys more
   often than the slave's, and bias the rolls and combos across the halves.

### Points to keep in mind

- **No tick wrap:** `ticks` is a `u64`, it would only wrap after about 584
  million years. Times are compared directly (`ticks >= next`), no
  `wrapping_*` needed. A `u32` wrapped after 49 days and froze the mouse and
  the LED blink.

<div align="center">
    <img src="../images/crab_back.png">
</div>

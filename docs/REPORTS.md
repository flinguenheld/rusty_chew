# Keyboard reports

How a pressed key becomes a USB report.

## Overview

```
 matrix ──► update_matrix ──► run() ──► key_buffer (queue) ──► USB loop ──► computer
 (keys)     pre-pressed →      hacks →   BuffCase, BuffCase…    1 report
            pressed            usb_code                         every ≥15 ms
```

The firmware **types**: each resolved key pushes one or more reports into a
queue, and the USB loop sends them one by one. It doesn't send "the keys held
right now" (see [Why not real NKRO](#why-not-real-nkro)).

Two rhythms:

- **Produce:** `chew.run` runs at each matrix update (every 5 ms on split,
  when the slave's keys arrive). It **pushes** cases into the queue.
- **Send:** the USB loop **pops** one case every `TIMER_USB_LOOP` (15 ms) at
  most.

The queue in between makes the hacks possible: one key can give a sequence of
reports, each with its own pause.

## 1. From the matrix to `pressed_keys`

`Chew::update_matrix` (`src/software/chew.rs`):

1. A new key enters `pre_pressed_keys` with `ticks: 1`.
2. Each update adds the elapsed ticks to every key (pre-pressed, pressed, home
   row).
3. After `COMBO_TIME` (20 ms) a pre-pressed key moves into `pressed_keys`.
   Until then, a combo can catch it.
4. Released keys and `KC::Done` keys are removed. `KC::DoneButKeep` stays
   (layer keys, mouse speed: they matter as long as they're held).
5. `mods.update_state` drops the modifiers whose key is released.

## 2. The hacks

`Chew::run`, in this order:

1. **Layout:** each pre-pressed key gets its `KC` from the current layer.
2. **Combos:** two pre-pressed keys matching `COMBOS` are both set to `Done`
   and replaced by one key in `pressed_keys`.
3. **Layers:** `Layout`, `LaySet`, `LayDead` set `layout.number`. A dead layer
   stays active for one key after its key is released.
4. **Leader key:** while active, letters fill `leader.buffer` and are set to
   `KC::None` (nothing typed). A matching sequence adds a key with
   `index: usize::MAX`.
5. **Dynamic macros:** record or replay.
6. **Modifiers:** `Alt` / `Altgr` / `Ctl` / `Gui` / `Sft` keep their **matrix
   index** in `Modifiers` (`src/software/modifiers.rs`), not a bool. A modifier
   is released when *its* key is released, even if the layer changed in
   between.
7. **Home row:** a home-row key waits in `homerow`:
   - held ≥ `HOLD_TIME` (150 ms) → becomes a modifier, the keys queued
     after it are typed with it,
   - released earlier → typed as its letter, with the queued keys after it.
   - While `homerow` isn't empty, the other keys queue there too, so the order
     is kept.
8. **Regular keys:** `usb_code` is called once per key, then the key is set to
   `Done`.

## 3. Building a report

`KC::usb_code` → `Buffer::add` (`src/software/keys.rs`).

One `BuffCase` = **one full HID report** (`key_code`, up to
`BUFFER_CASE_LENGTH` codes) **+ a `tempo`**, the pause to wait *after* sending
it.

```rust
key_code = mods.active() - excluded_mods  +  the key's own codes
```

| Key                 | Cases pushed |
|---------------------|--------------|
| `A`, Shift held     | `[LShift, A]` |
| `Num1`              | `[1]`, Shift excluded |
| `Exclaim`           | `[LShift, 1]` |
| `EAcute`            | `[RAlt, E]`, RAlt excluded from the mods and given explicitly |
| `ECircum` (ê)       | `[RAlt, 6]` + `TEMPO_DEAD_KEY` (50 ms) → `[mods, E]` → `[NoEvent]` |
| `Tion`              | `[T]` → `[I]` → `[O]` → `[N]` → `[NoEvent]`, Alt / Ctrl / Gui excluded so the letters can't become shortcuts |
| Text macro          | one case per character, `[NoEvent]` between two identical letters, a final `[NoEvent]` (`src/software/macros.rs`) |
| `KC::None`          | `[NoEventIndicated]` = everything released |

## 4. Holding and releasing

A report is **a state**, not an event: the computer keeps the last one until
the next one.

- **Holding a letter:** `[A]` stays the current report, the OS repeats it.
- `last_key` keeps **one** index, the last key typed:
  - released, no modifier held → `[NoEvent]` (everything released),
  - released, a modifier still held → `[mods]`. The letter is released and
    Ctrl stays, so Ctrl + click with the real mouse works.
- Pressing B while A is held: `[B]` replaces `[A]`, the computer sees A
  released. Nothing tracks A's own release, nothing is needed.

## 5. Sending

USB loop in `src/split.rs` (master) and `src/mono.rs`:

```rust
if usb_count_down.wait().is_ok() && key_buffer_tempo <= ticks {
    pop the first case
    if it differs from last_printed_key → write_report
```

| Result                        | What happens |
|-------------------------------|--------------|
| `Ok`                          | `key_buffer_tempo = ticks + tempo`, `last_printed_key = case` |
| `WouldBlock`, USB suspended   | remote wakeup, **queue emptied** (keys pressed to wake the PC are dropped) |
| `WouldBlock`                  | case put back at the front, next slot |
| `Duplicate`                   | blue LED, case dropped |

- **Mouse:** written straight away in the run, not through the queue.
- **`rusty_chew.tick()`:** every 1 ms, for the HID crate's own timing.
- **`SERIAL_ON`:** with the `serial` feature, no keyboard report is sent.

## Example: Shift held, then `ê`

| t      | Event                                  | Cases pushed                                    | Sent |
|--------|----------------------------------------|-------------------------------------------------|------|
| 0      | Shift pressed                          | (pre-pressed)                                   | |
| 25 ms  | Shift in `pressed_keys`, no `last_key` | `[LShift]` (active mods)                        | `[LShift]` |
| …      | `ê` pressed, then resolved             | `[RAlt, 6]` +50 ms, `[LShift, E]`, `[NoEvent]`  | `[RAlt, 6]` |
| +50 ms |                                        |                                                 | `[LShift, E]` → Ê |
| +15 ms |                                        |                                                 | `[NoEvent]` |
|        | Shift still held, `last_key` empty     | `[LShift]`                                      | `[LShift]` |

## Why not real NKRO

The USB device is already NKRO (`NKROBootKeyboard`). The limit is the model: a
report holds the modifiers + **one** key, so two letters can't be held
together (games, shortcuts made of two regular keys).

Real NKRO would send *modifiers + every held key* on each press and release.
That fights the hacks (dead keys, accents, macros, home-row keys resolved
late), which all need ordered sequences. They're used all day, so the model
stays.

If it's ever needed, a middle ground: only plain one-code keys (letters,
numbers, arrows) stay held together, everything else goes through the queue as
now.

## Points to keep in mind

Observations, not confirmed bugs.

- **Latency:**
  - a key waits more than `COMBO_TIME` before it resolves (about 25 ms on
    split, since `ticks` only move at each 5 ms exchange),
  - then up to `TIMER_USB_LOOP` for its USB slot,
  - so about 25-45 ms from press to report, + `HOLD_TIME` for home-row keys.
    HID allows a report every 1 ms, the 15 ms slot is a choice of this
    firmware.
- **Throughput:** one report per 15 ms is about 66 reports/s. `MacroNixOS`
  (≈ 45 reports) takes about 0.7 s, and keys typed meanwhile wait behind it.
- **Duplicate check:** `popped_key != last_printed_key` silently skips a case
  identical to the last one sent. That's why macros put `[NoEvent]` between
  two identical letters.
- **Silent drops:** `push_back(...).ok()` drops cases once the queue holds
  `BUFFER_LENGTH` (100).
- **Tick wrap:** `key_buffer_tempo <= ticks` doesn't handle the `ticks` wrap
  (after about 49 days on), unlike the rest of the code (`wrapping_*`).

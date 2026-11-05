<div align="center">
    <img src="../images/crab_coffee.png">
</div>

How a pressed key becomes a USB report.

### Overview

```
 matrix ──► update_matrix ──► run() ──► Nkro ──────► key_buffer (queue) ──► USB loop ──► computer
 (keys)     pre-pressed →      hacks     held keys   BuffCase, BuffCase…    1 report
            pressed                                                         ≥10 ms apart
```

The firmware does two things with the same queue:

- it **types**: a hack (accent, symbol, macro…) pushes one or more reports,
  and the USB loop sends them one by one,
- it **holds**: `Nkro` keeps the plain keys still down (letters, arrows,
  Enter…) and pushes the state (mods + held keys) each time it changes, so
  several keys can be held together (see [4](#4-held-keys-nkro)).

Two rhythms:

- **Produce:** `chew.run` runs at each matrix update (every 5 ms). It
  **pushes** cases into the queue.
- **Send:** the USB loop **pops** one case as soon as the last one is
  `TIMER_USB_LOOP` (10 ms) old.

> 💡 When all this happens, and what each timer costs: [TIMING.md](./TIMING.md)

<div align="center">
    <img src="../images/prawns.png">
</div>

### 1. From the matrix to `pressed_keys`

`Chew::update_matrix` (`src/software/chew.rs`):

1. A new key enters `pre_pressed_keys` with `ticks: 1`.
2. Each update adds the elapsed ticks to every key (pre-pressed, pressed, home
   row).
3. After `COMBO_TIME` (20 ms) a pre-pressed key moves into `pressed_keys`.
   Until then, a combo can catch it.
4. Released keys and `KC::Done` keys are removed. `KC::DoneButKeep` stays
   (layer keys, mouse speed: they matter as long as they're held).
5. `mods.update_state` drops the modifiers whose key is released.

> 💡 The matrix is read every 5 ms, what it means for `COMBO_TIME` and the
> combo window: [TIMING.md](./TIMING.md#the-5-ms-samples)

### 2. The hacks

`Chew::run`, in this order:

1. **Layout:** each pre-pressed key gets its `KC` from the current layer.
2. **Combos:** two pre-pressed keys matching `COMBOS` are both set to `Done`
   and replaced by one key in `pressed_keys`.
3. **Layers:** `Layout`, `LaySet`, `LayDead` set `layout.number`. A dead layer
   stays active for one key after its key is released.
4. **Leader key:** while active, letters fill `leader.buffer` and are set to
   `KC::None` (nothing typed). A matching sequence adds a key with
   `index: usize::MAX` (no matrix key, so it's a tap).
5. **Dynamic macros:** record or replay. A replay pushes its cases with
   `usb_code`, so it's a sequence: the held keys are emptied (`nkro.clear()`).
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
8. **Regular keys:** `nkro.type_key` is called once per key, then the key is
   set to `Done`.
9. **NKRO:** `nkro.update` removes the released keys and pushes the state if
   it changed.

### 3. Building a report

`KC::usb_code` → `Buffer::add` (`src/software/keys.rs`).

One `BuffCase` = **one full HID report** (`key_code`, up to
`BUFFER_CASE_LENGTH` = 16 codes).

```rust
key_code = mods.active() - excluded_mods  +  the key's own codes
```

| Key                 | Cases pushed |
|---------------------|--------------|
| `A`, Shift held     | `[LShift, A]`, by `Nkro` (+ the other held keys) |
| `Num1`              | `[1]`, Shift excluded |
| `Exclaim`           | `[LShift, 1]` |
| `EAcute`            | `[RAlt, E]`, RAlt excluded from the mods and given explicitly |
| `ECircum` (ê)       | `[RAlt, 6]` (dead key) → `[mods, E]` → `[NoEvent]` |
| `Tion`              | `[T]` → `[I]` → `[O]` → `[N]` → `[NoEvent]`, Alt / Ctrl / Gui excluded so the letters can't become shortcuts |
| Text macro          | one case per character, a final `[NoEvent]` (`src/software/macros.rs`) |
| `KC::None`          | `[NoEventIndicated]` = everything released |

<div align="center">
    <img src="../images/prawns.png">
</div>

### 4. Held keys (NKRO)

A report is **a state**, not an event: the computer keeps the last one until
the next one. `Nkro` (`src/software/nkro.rs`) decides which state it sees.

#### Three kinds of keys

A report has **one modifier byte for all its keys**. A key with its own
modifiers can't share a report: `!` = `[LShift, 1]` would turn a held `a`
into `A`.

| Kind | Which keys | Behaviour |
|---|---|---|
| **NKRO key** (`is_nkro_key`) | one code, no modifiers of its own: letters, Enter, Space, Esc, Del, Tab, Home / End / PageUp / PageDown, arrows, F1-F24 | **held together**, in `held` (up to `NKRO_MAX_KEYS` = 11) |
| **One-case hack** | a single case with excluded or added mods: `Num0`-`Num9`, symbols, `EAcute`, `CCedilla`, `STab`… (`k < KC::ACircum`) | held **alone** (`solo_key`) |
| **Sequence** | several cases ending with `[NoEvent]`: dead-key accents, `Qu`, `Tion`, text macros, dynamic macro replay (`k >= KC::ACircum`) | typed, never held |

The modifiers held by the user are not a problem: Ctrl + `a` + `s` =
`[LCtrl, A, S]`, the modifier is meant for all the keys.

#### `type_key`: when a key is typed

```mermaid
%%{init: {"flowchart": {"nodeSpacing": 30, "rankSpacing": 30}}}%%
flowchart TD
    K["type_key(code, index)"] --> N{"NKRO key?"}
    N -- yes --> D{"still down?"}
    D -- yes --> H["added at the end of held<br>push [state]"]
    D -- "no: tap" --> T{"already in held?"}
    T -- no --> T1["push [state + key]<br>push [state]"]
    T -- yes --> T2["push [state − key]<br>push [state]"]
    N -- no --> E["held emptied"]
    E --> S{"one-case hack<br>still down?"}
    S -- yes --> S1["solo_key = index<br>usb_code"]
    S -- no --> S2["usb_code<br>(sequence, tap)"]
```

`state` = `mods.active()` + the held keys, in the order they were pressed.

- **Held key:** a new key comes in its own case, after the keys already
  there. No timer groups keys: a report is an unordered bitmap, two keys
  merged in one case could come out in the wrong order on a roll.
- **Tap:** a key typed after its release (home-row tap, keys queued behind
  it) or without matrix key (leader result). It's added for one case, the held
  keys stay. A tap of a key already held releases it for one case, so it's
  typed once and stays held.
- **Hack or sequence:** `held` is emptied first. If the held keys stayed out of
  its cases only, the next state would add them back: the computer would see a
  second press of a key still held (`w!w`). So they stay out until pressed
  again.
- **NKRO key while a one-case hack is held:** it replaces the hack
  (`solo_key = None`).

#### `update`: end of each run

1. The last case pushed by this run is what the computer has now
   (`last_report`).
2. Released keys leave `held`, a released `solo_key` is cleared. The list
   keeps **matrix indexes**: a key leaves `pressed_keys` one run after it's
   typed (`Done`), the matrix says whether it's still down.
3. No `solo_key`: the state is pushed if it differs from `last_report`,
   `[NoEvent]` when empty.

The modifiers are in the state, so pressing or releasing one pushes a report
too. The mods stay without key: Ctrl + click with the real mouse works.

#### Examples

Hold `a`, then `s`:

```
 keys       reports       PC
 a down  →  [A]           a a a a…
 s down  →  [A, S]        s s s…  (the OS repeats the last key)
 a up    →  [S]
 s up    →  [NoEvent]
```

Hold `w`, tap `e` (home row, resolved on release):

```
 keys            reports         PC
 w down       →  [W]
 e down, up   →  [W, E], [W]     e typed, w still held
 w up         →  [NoEvent]
```

Hold `w`, then `!` (a sequence like `ê` works the same way):

```
 keys       reports          PC
 w down  →  [W]
 ! down  →  [LShift, 1]      held emptied: w released, ! typed
 ! up    →  [NoEvent]
 w up    →  nothing          w is already out
```

On split, `a` then `s`, both NKRO keys, no combo between them. The master
runs `update_matrix` + `run` every 5 ms:

| t | Matrix | `pre_pressed_keys` | `pressed_keys` | `held` | Pushed |
|---|---|---|---|---|---|
| 0 | a | a (1) | | | |
| 5-15 | a | a (6 → 16) | | | |
| 20 | a | ~~a~~ (Done) | a (A) → Done | A | `[A]` |
| 25 | a | | | A | |
| 30 | a, s | s (1) | | A | |
| 50 | a, s | ~~s~~ (Done) | s (S) → Done | A, S | `[A, S]` |
| 70 | s | | | S | `[S]` |
| 90 | | | | | `[NoEvent]` |

<div align="center">
    <img src="../images/prawns.png">
</div>

### 5. Sending

`UsbReports` (`src/software/usb_reports.rs`), called by the main loop of
`src/mono.rs` and of the split master (`src/split.rs`):

```rust
write_keyboard: if ticks reached next_ticks {
    pop the first case
    if it differs from last_printed_key → write_report
```

| Result                        | What happens |
|-------------------------------|--------------|
| `Ok`                          | `next_ticks = ticks + TIMER_USB_LOOP`, `last_printed_key = case` |
| `WouldBlock`, USB suspended   | remote wakeup, **queue emptied** (keys pressed to wake the PC are dropped) |
| `WouldBlock`                  | case put back at the front, tried again at the next loop pass |
| `Duplicate`                   | blue LED, case dropped |

- **Mouse:** `write_mouse`, straight away after the run, not through the queue. Red LED on `WouldBlock`.
- **`tick_hid`:** every 1 ms, for the HID crate's own timing.
- **`read_keyboard`:** after each USB poll, reads the computer's report (its LEDs, not used).
- **`SERIAL_ON`:** with the `serial` feature, no keyboard report is sent.

> 💡 A lone report goes out at once, the next ones wait for the gap:
> [TIMING.md](./TIMING.md#minimum-gap)

### Example: Shift held, then `ê`

```
 pushed in one run               written     PC
 [RAlt, 6]                       t           dead key ^, waits for a letter
 [LShift, E]                     t + 10      Ê
 [NoEvent]                       t + 20      everything released
 [LShift]      state, Shift held t + 30      Shift held again
```

Shift was already held, so `[LShift]` went out before. Each case waits the
10 ms gap.

**No pause after the dead key:** the OS composes the accent from the *order*
of the events, not their timing (`^` then `e` works however fast or slow it's
typed). What matters is that the computer reads the dead key's case on its
own, and the gap guarantees it: at most one report per host poll, and a
`WouldBlock` waits instead of being lost. A 50 ms `TEMPO_DEAD_KEY` was used
when the USB loop ran every 2 ms, faster than the host poll, and was removed
once tested useless.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Limits

- **Numbers and symbols** stay "one held key at a time" (they exclude or add
  Shift). Fine for typing, a limit for games that use numbers while moving.
  Possible later: a hack joins `held` when its modifiers are already the
  current ones (`Num1` with no Shift held, `!` while Shift is held).
- **Sequences don't repeat:** holding `ê` types one `ê`.
- **Combos:** two keys of a combo pressed within `COMBO_TIME` give the combo's
  key, not both. Pressed one after the other they're held together. For a
  game, use a layer without combos on those keys.
- **Two NKRO keys resolved in the same run** (within one 5 ms exchange on
  split): one case per key, in the order of `pressed_keys`, so their real
  order inside those 5 ms is unknown.
- **OS repeat:** with two keys held, the OS repeats only the last one pressed,
  and stops when it's released, even if the other is still down. Games read
  the keys down / up, not the repeat, so they see both.

### Points to keep in mind

Observations, not confirmed bugs.

- **Latency, throughput, timers:** see [TIMING.md](./TIMING.md#latency).
- **Duplicate check:** `popped_key != last_printed_key` silently skips a case
  identical to the last one sent (the HID crate refuses it too). So
  `Buffer::add` puts a `[NoEvent]` before a case identical to the last one
  queued: `[!] [NoEvent] [!]`. It covers two identical letters in a macro or a
  dynamic macro, and two `!` queued behind a home-row key and typed in the
  same run. NKRO states (`add_state`) skip it: a same state means the keys are
  still held.
- **Silent drops:** `push_back(...).ok()` drops cases once the queue holds
  `BUFFER_LENGTH` (100).

<div align="center">
    <img src="../images/crab_back.png">
</div>

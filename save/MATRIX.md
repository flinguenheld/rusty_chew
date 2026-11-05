<div align="center">
    <img src="../images/crab_coffee.png">
</div>

How the keys are numbered, how they are wired, and how the mono reads them.

### Key indexes

Each key has an index from 0 to 33, the same on mono and split. The layouts in
`src/layouts.rs` follow this order, and the chew logic only sees these indexes.

```
00  01  02  03  04    |    05  06  07  08  09
10  11  12  13  14    |    15  16  17  18  19
20  21  22  23        |        24  25  26  27
        28  29  30    |    31  32  33
```

Each scan gives the list of the pressed indexes. `Matrix` in
`src/hardware/matrix.rs` keeps the last two lists, so the chew logic can tell
a key just pressed (`freshly_pressed`) from a key still held (`is_active`).

```
scan n-1   [02]           ─┐
scan n     [02, 12]       ─┴─►  freshly_pressed = [12]    is_active(02) = true
```

<div align="center">
    <img src="../images/prawns.png">
</div>

### Two wirings

#### Split: direct pins

One pin per key, 17 per half. The pin has a pull-up: free it reads high,
pressed the switch ties it to the ground and it reads low. No scan, every pin
is read directly (`GpiosDirectPin` in `src/hardware/gpios.rs`).

```
3.3 V ── pull-up ──┬── GPx (input)
                   └── switch ── GND
```

#### Mono: a matrix

34 keys on 14 pins: 10 columns and 4 rows. Each key sits where a column
crosses a row, with a diode so the current only goes from the column to the
row.

```
      C0  C1  C2  C3  C4         C5  C6  C7  C8  C9
      |   |   |   |   |          |   |   |   |   |
R0 ── 00  01  02  03  04    |    05  06  07  08  09
R1 ── 10  11  12  13  14    |    15  16  17  18  19
R2 ── 20  21  22  23        |        24  25  26  27
R3 ──         28  29  30    |    31  32  33
```

| | Pins (`src/mono.rs`) | Mode |
|---|---|---|
| Columns C0 → C9 | GP28, GP27, GP26, GP15, GP14, GP4, GP3, GP2, GP1, GP0 | Outputs, low at rest |
| Rows R0 → R3 | GP5, GP6, GP7, GP8 | Inputs with pull-down |

One key, seen up close:

```
column ──►|── switch ──┬── row ── GPx (input)
       diode           └── pull-down (~50 kΩ) ── GND
```

- Free: the row only sees the pull-down, it reads low.
- Pressed while its column is high: the column feeds the row, it reads high.
- The diode stops a pressed key from feeding the other columns, so two keys
  held together can't light up a third one (no ghosting).

`CHEW_INDEXES` in `gpios.rs` gives the index of each column/row crossing,
`None` where there's no key (C4/R2, C0/R3…).

<div align="center">
    <img src="../images/prawns.png">
</div>

### Mono scan

Every `TIMER_MONO_LOOP` (5 ms), `GpiosMono::get_active_indexes` checks the
columns one by one:

```
for each column C0 → C9:
    1. column high
    2. wait 1 µs
    3. read the 4 rows: a high row = the key on this column/row is pressed
    4. column low
    5. wait for the 4 rows to go low (max ROWS_DISCHARGE_MAX µs)
```

With `c` (index 01) held, only C1 lights R0:

```
           C0   C1   C2   C3  …  C9
column     ▁▁▁▁ ▔▔▔▔ ▁▁▁▁ ▁▁▁▁   ▁▁▁▁
R0         ▁▁▁▁ ▔▔▔▔╲▁▁▁▁ ▁▁▁▁   ▁▁▁▁
                ▲ read: 01   ▲ read: nothing
```

A full scan takes a few dozen µs, a tiny part of the 5 ms loop. At most 16
pressed keys are returned (`Vec<u8, 16>`).

#### Step 5: why wait for the rows

When a column goes low, the diode blocks the way back: the row can only empty
through its weak pull-down, which takes a few µs. Without the wait, the next
column is read while the row is still high, and the key on its right shows up
too:

```
column C1  ▔▔▔▔▔▔╲▁▁▁▁▁▁▁▁▁▁▁▁▁
column C2  ▁▁▁▁▁▁▁▁▁▁▁▔▔▔▔▔▔▔▔▔▔
R0         ▔▔▔▔▔▔▔▔▔▔╲▁▁▁▁▁▁▁▁▁   ← still emptying
                       ▲ C2 read: R0 high → 02 'o' seen pressed
```

That was the `c → co`, `o → op`, `p → pv`… bug on the top row (R0).
`wait_rows_discharged` checks the rows every 1 µs and moves on once they are
all low, usually right away.

The wait is capped at `ROWS_DISCHARGE_MAX` (30 µs, `src/options.rs`): if a row
stays high (solder bridge, broken diode…), the scan still goes on. The keyboard
keeps working and the ghost letter shows which row is faulty. A normal row
empties in a few µs, so 30 leaves a wide margin and costs at most 0.3 ms per
scan.

### Points to keep in mind

- **More than 16 keys:** the extra ones are dropped by the scan. Far above
  `NKRO_MAX_KEYS` (11), so it never matters in practice.
- **The ghost comes back:** raise `ROWS_DISCHARGE_MAX` first, then look for a
  hardware problem on that row.

<div align="center">
    <img src="../images/crab_back.png">
</div>

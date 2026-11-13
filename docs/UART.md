<div align="center">
  <img src="./images/crab_coffee.png">
</div>

How a real serial port sends bytes: the **UART** (Universal Asynchronous
Receiver-Transmitter). The two halves of the split Chew talk this way.

The USB serial of the Chew only pretends to be one, see
[SERIAL_PC.md](./SERIAL_PC.md). The chip's vocabulary:
[RP2040.md](./RP2040.md).

### Serial and UART

**Serial** is the idea: the bits travel one after the other on one wire.
**UART** is one way to do it, with its own rules. Many buses are serial:

| Bus  | Wires                     | Clock                    | Unit on the wire        |
| ---- | ------------------------- | ------------------------ | ----------------------- |
| UART | TX, RX                    | none (agreed baud rate)  | 10-bit frame, 1 byte    |
| SPI  | clock, data in, data out… | a clock wire             | bits, as long as needed |
| I2C  | clock, data               | a clock wire             | address, then bytes     |
| USB  | D+, D-                    | hidden in the data       | packet, up to 64 bytes  |

The old "serial port" of PCs (RS-232) had a UART behind it. That's why
"serial" often means "UART".

### Serial vs Parallel

```text
 Serial: 1 wire, 8 bits in a row           Parallel: 8 wires, 1 bit each, at once

 TX ── b0 b1 b2 b3 b4 b5 b6 b7 ──>         D0 ── b0 ──>
       ──────── time ────────>             D1 ── b1 ──>
                                           ...
                                           D7 ── b7 ──>
```

| Serial                      | Parallel                                       |
| --------------------------- | ---------------------------------------------- |
| 1 wire per direction: cheap | 8 wires (or more)                              |
| works on long cables        | the bits don't arrive at the same time (skew)  |
| 1 bit per tick: slower      | fast, on short distances (inside a chip, a PC) |

<div align="center">
  <img src="./images/prawns.png">
</div>

### The Frame

The UART doesn't send a byte alone: it wraps it in a **frame**. One **bit**
before (start), one **bit** after (stop). Not bytes: a frame is 10 bits.

```text
 you give:          b7 b6 b5 b4 b3 b2 b1 b0        8 bits = 1 byte

 UART sends:  start b0 b1 b2 b3 b4 b5 b6 b7 stop   10 bits = 1 frame
                0                             1
```

The byte `'H'` (`0x48` = `0100 1000`) on the wire:

```text
 1  ────────────┐               ┌───┐       ┌───┐   ┌───────────
                │               │   │       │   │   │
 0              └───────────────┘   └───────┘   └───┘
                 0   0   0   0   1   0   0   1   0   1
                 St  b0  b1  b2  b3  b4  b5  b6  b7  Sp
      idle                                               idle
```

| Bit    | Level  | Why                                                  |
| ------ | ------ | ---------------------------------------------------- |
| idle   | 1      | nothing to send: the wire rests high                 |
| start  | 0      | the edge 1 → 0 says "a byte begins now"              |
| 8 data | 0 or 1 | the byte, lowest bit first                           |
| stop   | 1      | brings the wire back to idle, for the next start     |

The code never sees the start and stop bits: the UART hardware adds them on
one side and removes them on the other. The program only gives and gets bytes.

#### Idle, Start and Stop

The start and stop levels come from the idle level, not from the bit order:

- **Idle = 1**: a wire carrying current shows the line is alive. A cut wire
  reads 0 (a habit from the telegraph).
- **Start = 0**: it must differ from idle, to make an edge.
- **Stop = 1**: back to idle. Without it, after `0x00` (eight 0s) the wire
  would stay low, and the next start would make no edge.

#### Bit Order

The 8 data bits go **lowest bit first** (LSB first, "least significant
bit"). Only the data bits: the start and stop stay at their place.

```text
 0x48 written:   0 1 0 0 1 0 0 0      b7 … b0 (highest on the left)
 0x48 on wire:   0 0 0 1 0 0 1 0      b0 … b7 (in time order)
```

A historical choice that made the hardware simpler. The receiver puts the
bits back in order.

#### Frame Formats

The usual frame is **8N1**. Both sides must use the same.

| Name | Data bits | Parity | Stop bits | Bits per byte |
| ---- | --------- | ------ | --------- | ------------- |
| 8N1  | 8         | none   | 1         | 10            |
| 8E1  | 8         | even   | 1         | 11            |
| 8N2  | 8         | none   | 2         | 11            |

The **parity** bit is a simple error check: it makes the number of 1s even
(or odd). It finds one wrong bit, not two.

<div align="center">
  <img src="./images/prawns.png">
</div>

### Receiving

There is no clock wire: that's the **asynchronous** in UART. Both sides agree
on the speed in advance, and the receiver times each bit with its own clock.

```text
 1  ──┐   ┌───┐       ┌── ... ──┐   ┌──
      │   │   │       │         │   │
 0    └───┘   └───────┘         └───┘
      ↑ ↑   ↑   ↑   ↑             ↑
      │ │   └───┴───┴─ read each bit in the middle of its slot
      │ │                         └─ stop: must be 1
      │ └─ half a bit later: still 0? (a real start, not a glitch)
      └─ edge 1 → 0: start counting
```

| Step              | What the receiver does                                    |
| ----------------- | --------------------------------------------------------- |
| wait              | watches the wire, idle at 1                               |
| start edge        | the wire drops to 0: starts its timer                     |
| check the start   | half a bit later, still 0? Otherwise it was noise         |
| read 8 bits       | one every bit time, in the middle of the slot (stable)    |
| check the stop    | must be 1, otherwise a **framing error**                  |
| give the byte     | keeps the 8 data bits, drops start and stop               |

A **framing error** means the frame didn't end where expected: different
baud rates on the two sides, clocks too far apart, or noise on the wire.

#### One Sync per Byte

The two clocks are never exactly the same. Over 10 bits the drift is too small
to matter, over 1000 bits the receiver would read in the wrong place. Each
start edge syncs them again.

How far apart can they be? The last read (the stop bit) is 9.5 bits after the
start edge, and it must stay inside its slot: less than half a bit of drift.

```text
 allowed drift ≈ 0.5 bit ÷ 9.5 bits ≈ 5 %   (both sides together)
```

In practice, 2 to 3 % per side. A crystal is far below that, see
[CLOCKS.md](./CLOCKS.md#precision). The real killer is two different baud
rates: 9600 against 115200 is 12 times, not a few %.

Between two frames, the wire can stay idle for any time:

```text
 ──┐ St 'H' Sp ┌─────────────────────┐ St 'i' Sp ┌──
   └───────────┘   idle: any length  └───────────┘
```

<div align="center">
  <img src="./images/prawns.png">
</div>

### Speed

The **baud rate** is the number of bits per second on the wire, frame bits
included. With 8N1, a byte costs 10 bit times:

```text
 bytes per second = baud rate ÷ 10
```

| Baud rate | Bit time | Byte time | Bytes per second |
| --------- | -------- | --------- | ---------------- |
| 9600      | 104 µs   | 1.04 ms   | 960              |
| 115200    | 8.7 µs   | 87 µs     | 11 520           |
| 921600    | 1.1 µs   | 11 µs     | 92 160           |

20% of the line is framing: the price of having no clock wire.

### On the RP2040

| Way                  | What                                                   |
| -------------------- | ------------------------------------------------------ |
| `UART0`, `UART1`     | two hardware UARTs, each on a few possible pins       |
| PIO                  | a small program on a state machine, on any pin         |

Either way, the framing and the timing are done by the hardware: the code
gives bytes and gets bytes.

<div align="center">
  <img src="./images/crab_back.png">
</div>

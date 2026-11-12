<div align="center">
    <img src="../images/crab_coffee.png">
</div>

The serial inside the firmware: how any task prints a text or reads a line,
and what happens to the bytes on the way.

The USB setup and the tasks: [USB.md](./USB.md). Reading the serial from the
PC: [SERIAL_PC.md](./SERIAL_PC.md).

### The Big Picture

```
              SERIAL (static)
            ┌─────────────────┐
  any task ─┤ print()         │   channel_tx     serial_tx_run
            │   └─────────────┼─> [m][m][m]… ──> UsbSerialTx ──> PC
            │                 │
  any task <┤ read().await    │   channel_rx     serial_rx_run
            │   ▲─────────────┼── [m][m]…   <── UsbSerialRx <── PC
            └─────────────────┘                      │
                    ▲                                │ echo
                    └────────── print() ─────────────┘
```

Only two tasks touch the USB endpoints. The rest of the firmware uses
`SERIAL`, which holds one queue per direction.

| File | Holds |
|---|---|
| `src/usb/serial.rs` | `UsbSerial`: the two queues, `print` and `read` |
| `src/usb/serial_tx.rs` | `UsbSerialTx`: formats a message and sends it |
| `src/usb/serial_rx.rs` | `UsbSerialRx`: reads the packets, echoes, builds the lines |
| `src/usb/tasks.rs` | `serial_tx_run`, `serial_rx_run`: the two loops |
| `src/usb/types.rs` | `Message`, `MsgChannel`, the `SERIAL` static |
| `src/answerphone.rs` | A task that answers the commands typed |

### Using It

```rust
SERIAL.print(b"hello\n");          // from anywhere, async or not
let line = SERIAL.read().await;    // from a task
```

| Method | Waits | When it can't |
|---|---|---|
| `print(bytes)` | Never | Text over `SERIAL_MSG_MAX_LEN`: cut. Queue full: dropped |
| `read().await` | Until a full line arrives | — |

`print` never waits, so a debug text can't slow the keyboard down. The price:
it can be lost.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Messages and Queues

#### `Message`

A `Message` is a `heapless::Vec<u8, SERIAL_MSG_MAX_LEN>`: a length and a fixed
room of 200 bytes, with no heap.

```
  Message holding "hi\n"

  ┌─────┬───┬───┬────┬───┬───┬─────┬───┐
  │ len │ h │ i │ \n │ ? │ ? │  …  │ ? │
  │  3  │   │   │    │   │   │     │   │
  └─────┴───┴───┴────┴───┴───┴─────┴───┘
        └─ used (len) ─┘└─ reserved, never written ─┘
        └──────────── 200 bytes ───────────────────┘
```

The unused room is not initialised: creating a `Message` only sets `len`.
Filling it copies the bytes given, no more. Moving it (into a queue, out of a
queue) copies the whole struct.

#### The Queues

`MsgChannel<N>` is an `embassy_sync` `Channel` of `N` messages. It's protected
by a critical section, so it can be shared by all the tasks from a `static`
(see [EMBASSY.md](./EMBASSY.md#sharing-a-static)).

| Channel method | Queue full / empty | Used by |
|---|---|---|
| `try_send(m)` | Returns an error at once, the message is dropped | `print`, the lines received |
| `send(m).await` | Waits for a free slot | Not used |
| `receive().await` | Waits for a message | `read`, `serial_tx_run` |

#### The Sizes

All in `src/options.rs`:

| Constant | Value | Is |
|---|---|---|
| `SERIAL_PACKET_SIZE` | 64 | The biggest USB packet of the serial endpoints |
| `SERIAL_MSG_MAX_LEN` | 200 | The room of one `Message` |
| `SERIAL_TX_QUEUE_LEN` | 16 | Messages waiting to be sent |
| `SERIAL_RX_QUEUE_LEN` | 4 | Lines waiting to be read |

The two queues are 20 slots of about 200 bytes: 4 KB of RAM, always reserved.
Raising `SERIAL_MSG_MAX_LEN` grows every slot.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Sending

`serial_tx_run` loops on `UsbSerialTx::send_message`:

```
  channel_tx ──> format ──────────> write ──────────────────> PC
                 "\n" → "\r\n"      packets of 64 bytes
                                    + ZLP if the last is full
```

#### `\n` Becomes `\r\n`

A terminal needs two moves to start a new line. They come from the
teleprinters, where they were two mechanical actions:

| Byte | Name | Moves the cursor |
|---|---|---|
| `\r` | Carriage return | Back to the first column, same line |
| `\n` | Line feed | One line down, same column |

```
  "ab\ncd"          "ab\r\ncd"

  ab                ab
    cd              cd
```

The firmware writes `\n` only, `format` adds the `\r`. The copy is still a
`Message`: the 200 bytes are counted **after** the conversion, each line end
taking two. A `\r\n` that doesn't fit is left out as a whole, never a lone
`\r`.

#### Packets and the ZLP

`write_packet` sends one packet, 64 bytes at most. A longer message is cut in
chunks.

USB has no "end of transfer" mark: the PC knows it's over when a packet is
**shorter** than 64 bytes. After a full last packet it keeps waiting, so an
empty one is sent: the **ZLP** (Zero-Length Packet).

```
  message     packets sent

   70 bytes   [64] [6]         6 < 64: over
   64 bytes   [64] [0]         the ZLP says it's over
  128 bytes   [64] [64] [0]
    0 byte    [0]
```

What happens with no terminal open:
[USB.md](./USB.md#points-to-keep-in-mind).

<div align="center">
    <img src="../images/prawns.png">
</div>

### Receiving

`serial_rx_run` loops on `UsbSerialRx::read_packet`:

```
                         ┌──> echo ───────> SERIAL.print ──> the terminal
  PC ──> packet[..nb] ───┤    "\r" → "\n"
                         └──> push_byte × nb
                                └─ line end? ──> channel_rx ──> SERIAL.read
```

`packet` is a 64-byte array, `nb` the number of bytes really received. Both
branches work on `&packet[..nb]`: the rest of the array is zeros.

#### The Echo

A serial terminal doesn't show what is typed: it only sends it. The firmware
sends each packet back, so the text appears while typing.

Enter is sent as `\r`. Echoed as it is, the cursor would go back to the first
column and the answer would be written over the typed line. So `echo` turns
it into `\n`, which `format` sends as `\r\n`.

```
  typed     h  i  \r
  echoed    h  i  \n
  on wire   h  i  \r \n
```

The echo is one `print` per packet, not per byte: a pasted text takes one slot
of the TX queue, like a single key.

#### Building the Lines

A line can arrive in several packets (one per key when typing). The bytes are
kept in the `message` field between two `read_packet`:

```
  packets     [h]   [e]   [y]   [\r]            [o k \r]

  message     h     he    hey   (empty)         (empty)
                                  │                │
  channel_rx                      └─> "hey"        └─> "ok"
```

| Byte | Does |
|---|---|
| `\r` or `\n` | Ends the line: it goes to `channel_rx`, without the line end |
| Any other | Added to the line |

| Case | Result |
|---|---|
| Empty line (Enter alone, or `\r\n`) | Nothing is sent |
| Line over `SERIAL_MSG_MAX_LEN` | The extra bytes are dropped |
| `channel_rx` full (nobody reads) | The line is lost |

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Answerphone

`answerphone` (`src/answerphone.rs`) is a task that reads each line and
answers it.

```
  SERIAL.read ──> lower case ──> match ──┬─ "hello" ──> Hello!
                  (in place)             ├─ "test"  ──> Oh you want to test?
                                         └─ other   ──> the usage
```

- **Lower case in place**: `make_ascii_lowercase` changes the message itself,
  so `HeLLo` works and no second message is built.
- **`match` on `as_slice()`**: a pattern like `b"hello"` needs a slice, not a
  `Vec`. The comparison is exact, the line end is already removed.

#### The Usage

The usage is bigger than one message, so it's printed in three:

| Message | Bytes on the wire | Room left (of 200) |
|---|---|---|
| `USAGE_HEADER` | 143 | 57 |
| `USAGE_COMMANDS` | 46 | 154 |
| `USAGE_FOOTER` | 141 | 59 |

Three slots of the TX queue, out of 16. A new command only grows
`USAGE_COMMANDS`.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Points to Keep in Mind

- **A print can be lost**, silently: 16 messages waiting is the limit. Several
  prints in a row (a long usage, a loop) fill it fast.
- **Count the line ends twice** when a text gets close to 200 bytes.
- **No line editing.** Backspace is a byte like the others: it's added to the
  line, and the command won't match.
- **A terminal sending `\r\n` for Enter** gets two new lines in the echo. The
  line itself is right (the empty one is ignored).
- **The echo uses the TX queue**: with a full queue, typed keys still build
  the line but don't show up.

<div align="center">
    <img src="../images/crab_back.png">
</div>

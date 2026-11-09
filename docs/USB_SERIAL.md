<div align="center">
    <img src="../images/crab_coffee.png">
</div>

How to read what the Chew sends on its USB serial, from the PC. There is no
probe, so this is the debug output of the firmware.

How the serial is set up in the firmware: [USB.md](./USB.md).

### UART vs USB Serial

The PC sees `/dev/ttyACM0` as an old serial port ([UART](./UART.md)), but the bytes travel
in USB packets, not in UART frames:

| | UART | USB serial |
|---|---|---|
| Unit on the wire | 10-bit frame, holding 1 byte | packet, holding up to 64 bytes |
| Start mark | 1 start bit | a SYNC pattern (8 bits) |
| End mark | 1 stop bit | an "end of packet" signal |
| Error check | none (or 1 parity bit) | a checksum (CRC) in each packet |
| Speed | agreed in advance (baud rate) | fixed: 12 Mbit/s on the RP2040 |
| Who talks | either side, any time | only when the PC asks |

The baud rate set in the terminal is sent to the Chew, stored by
`embassy-usb` (`line_coding`), and ignored.

<div align="center">
    <img src="../images/prawns.png">
</div>

### From the Firmware to the Terminal

```
  Chew                                   PC (Linux)
  ┌──────────────────────┐               ┌───────────────────────────────────┐
  │ test_serial task     │               │ cdc_acm (kernel driver)           │
  │   class.write_packet │ ── USB ────>  │   └─> /dev/ttyACM0                │
  │ usb_run task         │   packets     │         ├─> cat                   │
  └──────────────────────┘               │         └─> tio                   │
                                         └───────────────────────────────────┘
```

The board says it's a **CDC ACM** device (USB serial). Linux loads its
`cdc_acm` driver and creates a `/dev/ttyACM*` file: reading this file gives
the bytes the firmware sends.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Finding the Port

`ttyACM*` names are given in plug order to **any** USB serial device (another
microcontroller, a modem, a 3D printer…). `/dev/ttyACM0` exists only proves
that one is plugged in, not that it's the Chew.

The host reads the device's names from its descriptors, set in `main` by the
`Config` (see [USB.md](./USB.md#config)):

| `Config` field | Value | Seen as |
|---|---|---|
| `Config::new(vid, pid)` | `0xc0de`, `0xcafe` | `idVendor`, `idProduct` |
| `manufacturer` | `florent@linguenheld.net` | `Manufacturer` |
| `product` | `Chew embassy` | `Product` |
| `serial_number` | `0001` | `SerialNumber` |

#### `/dev/serial/by-id/`

The clearest: udev makes a link named after these strings.

```
ls -l /dev/serial/by-id/
# usb-<manufacturer>_<product>_0001-if00 -> ../../ttyACM0
```

```
  usb - <manufacturer> _ <product> _ 0001 - if00
        │                │           │      └─ interface 0 (the CDC control interface)
        │                │           └─ serial_number
        │                └─ product
        └─ manufacturer
```

udev replaces the spaces (and other unsafe characters) with `_`, so
`Chew embassy` becomes `Chew_embassy`. The exact name: run the `ls` above.

This link can be opened directly. It stays the same when another device takes
`ttyACM0`, so it's the safest name to use.

#### The Kernel Log

Right after plugging in:

```
sudo dmesg | tail
# usb 1-2: New USB device found, idVendor=c0de, idProduct=cafe
# usb 1-2: Product: Chew embassy
# usb 1-2: Manufacturer: florent@linguenheld.net
# cdc_acm 1-2:1.0: ttyACM0: USB ACM device
```

#### udev

What udev knows about one port:

```
udevadm info /dev/ttyACM0 | grep -E 'ID_VENDOR|ID_MODEL|ID_SERIAL'
```

#### Other Checks

| Command | Shows |
|---|---|
| `ls -l /dev/ttyACM*` | The USB serial ports, and their group (`dialout`) |
| `lsusb \| grep c0de` | The Chew is enumerated (even without a serial port) |

<div align="center">
    <img src="../images/prawns.png">
</div>

### Access to the Port

```
crw-rw----. 1 root dialout 166, 0 Nov  5 17:56 /dev/ttyACM0
```

Only `root` and the `dialout` group can open it. Either use `sudo`, or join the
group once.

On Fedora Atomic the system groups are in `/usr/lib/group` (read only), so
`usermod` can't find `dialout`. Copy its line into `/etc/group` first:

```
grep -E '^dialout:' /usr/lib/group | sudo tee -a /etc/group
sudo usermod -aG dialout $USER
```

The new group applies at the next login.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Reading with `cat`

```
sudo cat /dev/ttyACM0      # quit: Ctrl-c
```

Nothing to install, good for a first check. But it only reads, and it exits
when the port disappears.

Opening the port is also what lets the firmware go on: `write_packet` waits
until the PC reads, so with no `cat` or `tio` open, `test_serial` pauses (see
[USB.md](./USB.md#points-to-keep-in-mind)).

### Reading with `tio`

```
rpm-ostree install tio     # once, then reboot
tio /dev/ttyACM0           # quit: Ctrl-t q
tio /dev/serial/by-id/usb-*Chew*   # the by-id link, whatever its exact name
```

`tio` **reconnects by itself**. To flash, the board goes in BOOTSEL mode (a USB
drive), then `cargo run` writes it and it restarts the firmware: the port
disappears, then comes back. `tio` waits and
reopens it, so it can stay open while flashing.

```
 time ──────────────────────────────────────────────────────────────►

 board    firmware │ BOOTSEL (flashing) │ firmware again
 port     ttyACM0  │      (none)        │ ttyACM0
 cat      reading  ✗ exits
 tio      reading  │ waiting…           │ reading
```

#### The Baud Rate Doesn't Matter

On a real UART the speed is the wire's timing, both sides must agree. CDC ACM
is USB packets: the "baud rate" the terminal sets is only a value sent to the
device, which the firmware ignores. Any value works.

#### ModemManager

`ModemManager` (running on Fedora) probes each new `ttyACM*` for a few seconds,
sending modem commands to it. The firmware ignores them, but the port can be
"busy" just after plugging in. If that gets annoying, a udev rule with
`ENV{ID_MM_DEVICE_IGNORE}="1"` for `c0de:cafe` tells it to skip the Chew.

<div align="center">
    <img src="../images/crab_back.png">
</div>

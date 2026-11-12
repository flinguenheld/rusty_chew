<div align="center">
    <img src="../images/crab_coffee.png">
</div>

The USB side of the Chew: how the PC sees it, and how `embassy-usb` is set up
and run. Today it's a USB serial (CDC ACM), the keyboard (HID) will join it.

The setup is in `src/mono.rs`, the buffers in `src/usb/device.rs`, the statics
and type aliases in `src/usb/types.rs`. The serial inside the firmware (queues,
lines, echo): [SERIAL.md](./SERIAL.md). Reading it from the PC:
[SERIAL_PC.md](./SERIAL_PC.md). The interrupt: [INTERRUPTS.md](./INTERRUPTS.md).
The chip's words (peripheral, FIFO…): [RP2040.md](./RP2040.md).

### Host and Device

On USB the PC is the **host**: it decides everything, it asks and the Chew
answers. The Chew is a **device**: it never talks first, it only replies, or
fills a buffer that the host comes to read.

```
  PC (host)                               Chew (device)
  ┌──────────────┐   asks: "who are you?"   ┌──────────────┐
  │              │ ───────────────────────> │              │
  │              │ <─────────────────────── │  descriptors │
  │              │   reads: "any data?"     │              │
  │              │ ───────────────────────> │              │
  │              │ <─────────────────────── │  "hello\r\n" │
  └──────────────┘                          └──────────────┘
```

So the Chew must **always be ready to answer**. If it stops answering, the PC
drops it.

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Layers

```
  our code        SERIAL.print(b"hello\n")
                  serial_tx_run: tx.write_packet
                        │
  class           CdcAcmClass  (embassy_usb::class::cdc_acm)  ← usbd_serial::SerialPort
                        │
  device          UsbDevice    (embassy_usb)                  ← usb_device::UsbDevice
                  usb_run: usb.run(), answers the host
                        │
  driver          usb::Driver<USB>  (embassy_rp)              ← hal::usb::UsbBus
                        │                                        + UsbBusAllocator
  interrupt       USBCTRL_IRQ → usb::InterruptHandler<USB>    ← our #[interrupt]
                                                                 + usb_dev.poll()
```

| Layer | Type | Job |
|---|---|---|
| Driver | `usb::Driver<'static, USB>` (alias `UsbDriver`) | The RP2040 USB hardware: endpoints, packets |
| Device | `UsbDevice` | The USB protocol: enumeration, descriptors, setup requests |
| Class | `CdcAcmClass` | One function of the device: here a serial port. HID will be another |

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Setup

Done once in `main`, in this order:

```
  1. Driver::new(p.USB, Irqs)        the hardware
  2. Config::new(vid, pid)           what the PC sees
  3. USB_BUFFERS.init(…)             memory for embassy-usb
         │
         ▼
     Builder::new(driver, config, buffers)
         │
  4.     ├── CdcAcmClass::new(&mut builder, …)   adds its interfaces & endpoints
         │     └── split() ──> sender, receiver
         │   (HID: another class here, later)
         │
  5.     └── builder.build() ──> UsbDevice       descriptors frozen
                                    │
  6. spawn usb_run(usb)  ◄──────────┘
     spawn serial_rx_run(receiver)
     spawn serial_tx_run(sender)
```

The `Builder` collects the classes, then `build` writes the final descriptors.
After `build`, no class can be added.

#### `Config`

| Field | Value | Seen by the PC as |
|---|---|---|
| `Config::new(vid, pid)` | `0xc0de`, `0xcafe` | `idVendor`, `idProduct`: test values, not registered |
| `manufacturer` | `florent@linguenheld.net` | `Manufacturer` |
| `product` | `Chew embassy` | `Product` |
| `serial_number` | `1506` | `SerialNumber` |

These strings are readable by any PC the Chew is plugged into (`lsusb -v`,
`dmesg`).

`Config::new` makes a **composite device** by default (class `0xEF/0x02/0x01`,
`composite_with_iads: true`): several functions in one device, like the serial
and the keyboard later. `Builder::new` panics if the class is changed without
turning the IADs off.

#### The Buffers

`embassy-usb` has no heap: it can't allocate, so it borrows arrays we give it.
They are one struct, `UsbBuffers` (`src/usb/device.rs`), in one `StaticCell`,
`USB_BUFFERS` (`src/usb/types.rs`), so they live forever and the device can go
in a task (see
[EMBASSY.md](./EMBASSY.md#static-and-staticcell)).

| Field | Size | Written by | Holds |
|---|---|---|---|
| `config_descriptor` | 256 | `Builder`, the classes | The description sent to the PC: interfaces, endpoints |
| `bos_descriptor` | 256 | `Builder` | Extra USB 2.1 capabilities |
| `control_buf` | 64 | `run()` | One host request, while it's handled |
| `state` | — | `CdcAcmClass` | Not a buffer: variables shared by the class and `run()` (DTR, baud rate) |

The MS OS descriptors (Windows only) get `&mut []`, an empty slice.

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Tasks

```
 executor
   ├── usb_run        usb.run()                forever, woken by USBCTRL_IRQ
   ├── serial_rx_run  loop: rx.read_packet     PC ──> channel_rx
   ├── serial_tx_run  loop: tx.send_message    channel_tx ──> PC
   └── answerphone    loop: SERIAL.read        answers each line
                            SERIAL.print
```

#### `usb_run`: Always Answering

```rust
#[embassy_executor::task]
async fn usb_run(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
}
```

`run()` never returns (`-> !`). It waits for a USB event, handles it, and
waits again. It's the async version of rp-hal's `usb_dev.poll()`: no need to
call it often, `USBCTRL_IRQ` wakes it.

Its main job is the **enumeration**, right after plugging in:

```
 PC                                           run() on the Chew
  │── bus reset ──────────────────────────────>│
  │── GET_DESCRIPTOR (device) ────────────────>│ c0de:cafe, composite
  │── SET_ADDRESS ────────────────────────────>│
  │── GET_DESCRIPTOR (config) ────────────────>│ config_descriptor
  │── GET_DESCRIPTOR (strings) ───────────────>│ manufacturer, product, serial
  │── SET_CONFIGURATION ──────────────────────>│ endpoints enabled
  │                                            │   └──> wait_connection() returns
  │── SET_LINE_CODING, SET_CONTROL_LINE_STATE >│ serial settings, DTR
```

Without `usb_run`, nobody answers: `dmesg` shows `device descriptor read
error`, and no `/dev/ttyACM*` appears.

#### The Serial Tasks

```rust
#[embassy_executor::task]
pub async fn serial_tx_run(sender: Sender<'static, UsbDriver>) {
    let mut tx = UsbSerialTx::new(sender);

    loop {
        tx.send_message().await;
    }
}
```

`serial_rx_run` has the same shape, with `rx.read_packet()`. Both are in
`src/usb/tasks.rs`. They are the only ones to touch the serial endpoints: the
rest of the firmware uses the `SERIAL` static, which holds a queue (a channel)
for each direction.

```
  any task ── SERIAL.print() ──> channel_tx ──> serial_tx_run ──> PC
  any task <── SERIAL.read() ─── channel_rx <── serial_rx_run <── PC
```

What happens to the bytes in each direction, and the `answerphone` task:
[SERIAL.md](./SERIAL.md).

#### Each Task Its Endpoint

The tasks don't use the same **endpoints** (the "mailboxes" of a USB device):

```
                 ┌── EP0 (control) ──── usb_run        descriptors, setup, DTR
  USB cable ─────┼── bulk IN ────────── serial_tx_run  the serial data, to the PC
                 └── bulk OUT ───────── serial_rx_run  the serial data, from the PC
```

They work on their own, but the serial tasks need `usb_run`: the bulk
endpoints are only enabled after `run()` has handled `SET_CONFIGURATION`.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Points to Keep in Mind

- **`write_packet` sends one packet: 64 bytes at most.** Longer data returns
  `Err(BufferOverflow)`. The splitting:
  [SERIAL.md](./SERIAL.md#packets-and-the-zlp).
- **`wait_connection` waits for the configuration, not for a terminal.** On
  Linux, `write_packet` then waits until something reads the port: without a
  terminal open, `serial_tx_run` pauses and the TX queue fills, then drops
  ([SERIAL.md](./SERIAL.md#using-it)). `sender.dtr()` tells when a terminal
  has opened it.
- **Errors are ignored** (`.ok()`): when the cable is pulled, the next message
  waits in `wait_connection`.
- **The USB needs exactly 48 MHz** (±0.25 %): `clk_usb`, from its own PLL
  ([CLOCKS.md](./CLOCKS.md)). A wrong clock and the PC rejects the device.
- **`embassy-usb` without default features**: it drops `usbd-hid`, which HID
  will need.

<div align="center">
    <img src="../images/crab_back.png">
</div>

<div align="center">
    <img src="../images/crab_coffee.png">
</div>

The USB side of the Chew: how the PC sees it, and how `embassy-usb` is set up
and run. Today it's a USB serial (CDC ACM), the keyboard (HID) will join it.

The setup is in `src/mono.rs`, the buffers in `src/usb.rs`. Reading the serial
from the PC: [USB_SERIAL.md](./USB_SERIAL.md). The interrupt: [INTERRUPTS.md](./INTERRUPTS.md).
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
  our code        test_serial: class.write_packet(b"hello\r\n")
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
         │   (HID: another class here, later)
         │
  5.     └── builder.build() ──> UsbDevice       descriptors frozen
                                    │
  6. spawn usb_run(usb)  ◄──────────┘
     spawn test_serial(class)
```

The `Builder` collects the classes, then `build` writes the final descriptors.
After `build`, no class can be added.

#### `Config`

| Field | Value | Seen by the PC as |
|---|---|---|
| `Config::new(vid, pid)` | `0xc0de`, `0xcafe` | `idVendor`, `idProduct`: test values, not registered |
| `manufacturer` | `florent@linguenheld.net` | `Manufacturer` |
| `product` | `Chew embassy` | `Product` |
| `serial_number` | `0001` | `SerialNumber` |

These strings are readable by any PC the Chew is plugged into (`lsusb -v`,
`dmesg`).

`Config::new` makes a **composite device** by default (class `0xEF/0x02/0x01`,
`composite_with_iads: true`): several functions in one device, like the serial
and the keyboard later. `Builder::new` panics if the class is changed without
turning the IADs off.

#### The Buffers

`embassy-usb` has no heap: it can't allocate, so it borrows arrays we give it.
They are in one `StaticCell`, `USB_BUFFERS` (`src/usb.rs`), so they live
forever and the device can go in a task (see
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
   ├── usb_run      usb.run()                   forever, woken by USBCTRL_IRQ
   └── test_serial  loop: wait_connection
                          write_packet "hello"
                          Timer 1 s
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

#### `test_serial`: Sending

```rust
#[embassy_executor::task]
async fn test_serial(mut class: CdcAcmClass<'static, UsbDriver>) {
    loop {
        class.wait_connection().await;
        let _ = class.write_packet(b"hello\r\n").await;
        Timer::after_millis(1000).await;
    }
}
```

A test loop, like `test_led`: it will be replaced by a `Serial` struct.

#### Two Tasks, Two Paths

`run()` and `write_packet` don't use the same **endpoints** (the "mailboxes"
of a USB device):

```
                 ┌── EP0 (control) ──── usb_run      descriptors, setup, DTR
  USB cable ─────┤
                 └── bulk IN / OUT ──── test_serial  the serial data
```

They work on their own, but `test_serial` needs `usb_run`: the bulk endpoints
are only enabled after `run()` has handled `SET_CONFIGURATION`.

<div align="center">
    <img src="../images/prawns.png">
</div>

### Points to Keep in Mind

- **`write_packet` sends one packet: 64 bytes at most.** Longer data returns
  `Err(BufferOverflow)`. Longer text must be split, it will be the job of the
  `Serial` struct.
- **`wait_connection` waits for the configuration, not for a terminal.** On
  Linux, `write_packet` then waits until something reads the port: without a
  terminal open, `test_serial` pauses. `class.dtr()` tells when a terminal has
  opened it.
- **Errors are ignored** (`let _ =`): when the cable is pulled, the loop goes
  back to `wait_connection`.
- **The USB needs exactly 48 MHz** (±0.25 %): `clk_usb`, from its own PLL
  ([CLOCKS.md](./CLOCKS.md)). A wrong clock and the PC rejects the device.
- **`embassy-usb` without default features**: it drops `usbd-hid`, which HID
  will need.

<div align="center">
    <img src="../images/crab_back.png">
</div>

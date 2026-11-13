<div align="center">
    <img src="./images/crab_coffee.png">
</div>

How the async drivers get woken up: the interrupts, and `bind_interrupts!`.
The executor that sleeps and polls the tasks is in [EMBASSY.md](./EMBASSY.md).

### The Vector Table

The words used here (peripheral, driver, IRQ, NVIC…) are explained in
[RP2040.md](./RP2040.md).

The vector table is a list of addresses at the start of the firmware (in flash, just after boot2). When
an interrupt fires, the CPU itself reads the matching entry and jumps to that
address: the entry tells it **which function to run**.

```
  flash 0x1000_0100   vector table
  ┌─────┬─────────────────┬──────────────────────────────┐
  │ 0   │ initial stack   │                              │
  │ 1   │ Reset           │ → start of the firmware      │
  │ 2.. │ NMI, HardFault… │ CPU exceptions               │
  ├─────┼─────────────────┼──────────────────────────────┤
  │ 16  │ TIMER_IRQ_0     │ → embassy-rp time driver     │  IRQ 0
  │ 21  │ USBCTRL_IRQ     │ → usb::InterruptHandler      │  IRQ 5
  │ 23  │ PIO0_IRQ_0      │ → pio::InterruptHandler      │  IRQ 7
  │ 27  │ DMA_IRQ_0       │ → dma::InterruptHandler      │  IRQ 11
  │ 29  │ IO_IRQ_BANK0    │ → embassy-rp GPIO handler    │  IRQ 13
  │ ... │ (others)        │ → DefaultHandler (unused)    │
  └─────┴─────────────────┴──────────────────────────────┘

  DMA done ──> IRQ 11 ──> CPU reads entry 27 ──> jumps to DMA_IRQ_0() ──> wakes the future
```

`cortex-m-rt` builds this table at link time: an entry gets a function if one has
the exact name (`DMA_IRQ_0`), else `DefaultHandler`. There is one table per
firmware, so one function per line. `bind_interrupts!` writes these functions.

<div align="center">
    <img src="./images/prawns.png">
</div>

### One Table for the Whole Firmware

The RP2040 has **one vector table**: a fixed list of interrupt lines (`PIO0_IRQ_0`,
`DMA_IRQ_0`, `USBCTRL_IRQ`, `UART0_IRQ`, …), each with **one handler** for the whole
program. `bind_interrupts!` fills in this table, so it belongs to the firmware, not to
one driver: a single `Irqs` in `main`, that every driver uses.

```
             vector table (one per chip)
             ┌──────────────┐
  Irqs ────> │ PIO0_IRQ_0   │──> PIO handler   ──> wakes the state machine futures
             │ DMA_IRQ_0    │──> DMA handler   ──> wakes led.write()
             │ TIMER_IRQ_0  │──> set by embassy-rp itself (time driver), not in Irqs
             └──────────────┘
```

The lines bound by `Irqs` in `src/mono.rs`:

| Line | Handler | Used by |
|---|---|---|
| `PIO0_IRQ_0` | `pio::InterruptHandler<PIO0>` | `Pio::new` (the PIO block of the LED) |
| `DMA_IRQ_0` | `dma::InterruptHandler<DMA_CH0>` | `PioWs2812` (LED colors pushed by DMA) |
| `USBCTRL_IRQ` | `usb::InterruptHandler<USB>` | `usb::Driver::new` (the USB device, see [USB.md](./USB.md)) |

A driver that needs another interrupt adds its line to the same `Irqs`.

Every driver gets the same value, `Irqs`, and only checks that **its** line is in it:

```
                   ┌──> Pio::new(p.PIO0, Irqs)                  needs PIO0_IRQ_0
  struct Irqs ─────┤
                   └──> PioWs2812::…(…, p.DMA_CH0, Irqs, …)     needs DMA_IRQ_0
```

<div align="center">
    <img src="./images/prawns.png">
</div>

### `bind_interrupts!`

#### The Problem It Solves

An async driver doesn't spin on a register. It starts the hardware, then its future
returns `Pending` and the executor goes to sleep (`WFE`). Something has to wake the
future when the hardware is done: that something is an **interrupt**.

```
 task                    driver                  hardware           interrupt handler
  │  led.write().await     │                        │                       │
  │───────────────────────>│  start DMA transfer    │                       │
  │                        │───────────────────────>│                       │
  │       Pending          │                        │                       │
  │<───────────────────────│                        │                       │
  │  (executor sleeps)     │                        │  transfer done        │
  │                        │                        │──── DMA_IRQ_0 ───────>│
  │                        │                        │                       │ waker.wake()
  │  polled again → Ready  │<───────────────────────────────────────────────│
```

So each driver needs its interrupt handler to be installed in the vector table.
Without `bind_interrupts!`, it's done by hand:

```rust
#[interrupt]
fn PIO0_IRQ_0() { /* read the flags, do the work */ }

unsafe { pac::NVIC::unmask(pac::Interrupt::PIO0_IRQ_0) };
```

Nothing linked the handler to the driver: forget it and the firmware still builds,
then hangs (or does the default handler) at runtime.

#### What We Write

```rust
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});
```

Read it as: *"interrupt line `PIO0_IRQ_0` runs the PIO0 handler, interrupt line
`DMA_IRQ_0` runs the DMA channel 0 handler"*. The handlers are written by embassy-rp,
we only choose which line runs which.

#### What the Macro Generates

1. An empty type, the "proof" token:

```rust
#[derive(Copy, Clone)]
struct Irqs;
```

2. One real interrupt handler per line, with the exact name of the vector
   (`#[no_mangle]`, so `cortex-m-rt` puts it in the vector table):

```rust
#[no_mangle]
unsafe extern "C" fn PIO0_IRQ_0() {
    <pio::InterruptHandler<PIO0> as Handler<PIO0_IRQ_0>>::on_interrupt();
}

#[no_mangle]
unsafe extern "C" fn DMA_IRQ_0() {
    <dma::InterruptHandler<DMA_CH0> as Handler<DMA_IRQ_0>>::on_interrupt();
}
```

3. One trait impl per handler, the compile-time promise "`Irqs` binds this line to
   this handler":

```rust
unsafe impl Binding<PIO0_IRQ_0, pio::InterruptHandler<PIO0>> for Irqs {}
unsafe impl Binding<DMA_IRQ_0, dma::InterruptHandler<DMA_CH0>> for Irqs {}
```

#### Why the Drivers Ask for `Irqs`

The constructors take the token as an argument:

```rust
// embassy-rp
pub fn new(pio: Peri<'d, PIO>, irq: impl Binding<PIO::Interrupt, InterruptHandler<PIO>>) -> Self
```

```rust
Pio::new(p.PIO0, Irqs);                                  // needs Binding<PIO0_IRQ_0, …>
PioWs2812::with_color_order(…, p.DMA_CH0, Irqs, …);      // needs Binding<DMA_IRQ_0, …>
```

The value `Irqs` carries no data, only its type matters: if a line is missing or
binds the wrong handler, the trait impl doesn't exist and **the build fails**. The
forgotten handler becomes a compile error instead of a hang at runtime.

The constructor also enables the interrupt in the NVIC (`Interrupt::enable()`), no
more `NVIC::unmask` by hand.

```
bind_interrupts!            Irqs (type)                 driver::new(…, Irqs)
  ├─ fn PIO0_IRQ_0()  ──>   impl Binding<PIO0_IRQ_0,…>  ──>  Pio::new        ✓ compiles
  └─ fn DMA_IRQ_0()   ──>   impl Binding<DMA_IRQ_0,…>   ──>  PioWs2812::new  ✓ compiles
                                                              (missing line → ✗ error)
```

#### What the Handlers Do

| Line | Handler | When it fires | What it does |
|---|---|---|---|
| `PIO0_IRQ_0` | `pio::InterruptHandler<PIO0>` | a state machine FIFO has room / data, or a PIO IRQ flag | wakes the futures waiting on that state machine, clears the enable bits |
| `DMA_IRQ_0` | `dma::InterruptHandler<DMA_CH0>` | the DMA transfer of channel 0 is done | wakes the future of `led.write()` (panics on a bus error) |
| `USBCTRL_IRQ` | `usb::InterruptHandler<USB>` | bus reset, suspend, resume, a setup request, an endpoint buffer done | wakes `usb.run()` (bus events, setup on EP0) or the endpoint futures (`write_packet`…) |

`PioWs2812::write` pushes the color words with DMA, so the LED only really waits on
`DMA_IRQ_0`. `Pio::new` still asks for the PIO binding: it sets up the whole
PIO block, and other programs on it can wait on their FIFOs or IRQ flags.

<div align="center">
    <img src="./images/prawns.png">
</div>

### Rules

#### Several Handlers on One Line

On the RP2040 all DMA channels share `DMA_IRQ_0`. When another channel is used (UART,
…), it goes on the same line, comma separated:

```rust
DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
```

The generated `DMA_IRQ_0()` calls both handlers, each checks its own channel bit.

#### One Line, One Place

Only one `bind_interrupts!` can define a given line: two would mean two functions
named `DMA_IRQ_0`, a link error. Several `bind_interrupts!` structs are possible if
they bind different lines, but one `Irqs` in `main` keeps the whole table in one
place.

<div align="center">
    <img src="./images/crab_back.png">
</div>

<div align="center">
    <img src="../images/crab_coffee.png">
</div>

What embassy is in this firmware: the crates, `init`, and the executor that runs
the `async` tasks.

How it's built and flashed: [BUILD.md](./BUILD.md). How the drivers get woken
up: [INTERRUPTS.md](./INTERRUPTS.md).

### The Layers

```
  our code         tasks: test_led, …            (src/mono.rs, src/software/)
                         │ .await
  executor         embassy-executor              runs the tasks, sleeps (WFE)
                         │
  HAL              embassy-rp                    async drivers: PIO, DMA, USB…
                   embassy-time                  Timer, Ticker, Instant
                         │ registers, interrupts
  hardware         RP2040                        Cortex-M0+, PIO0, DMA, timer…
```

With rp-hal, the HAL only gave blocking drivers, and the main loop polled
everything in turn. With embassy, each part is a task that `.await`s its
hardware, and the executor runs whichever task is ready.

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Crates

```toml
[dependencies]
embassy-executor = { version = "0.10", features = ["platform-cortex-m", "executor-thread"] }
embassy-rp = { version = "0.10", features = ["rp2040", "time-driver", "critical-section-impl"] }
embassy-time = "0.5"

cortex-m-rt = "0.7"

defmt = "1"
defmt-rtt = "1"
panic-probe = { version = "1", features = ["print-defmt"] }
```

| Crate / feature | Role |
|---|---|
| `embassy-executor` | Runs the `async` tasks |
| `platform-cortex-m` | Executor for Cortex-M (the RP2040 is a Cortex-M0+) |
| `executor-thread` | The executor runs in thread mode, sleeps (`WFE`) when no task has work |
| `embassy-rp` | The HAL, replaces `rp2040-hal` / `waveshare-rp2040-zero` |
| `rp2040` | The chip (embassy-rp also supports the RP235x) |
| `time-driver` | The RP2040 hardware timer is the time source of `embassy-time` |
| `critical-section-impl` | The lock used by embassy, with the RP2040 hardware spinlocks (2 cores) |
| `embassy-time` | `Timer`, `Ticker`, `Instant`, `Duration` |
| `cortex-m-rt` | Reset & vector table, but `#[embassy_executor::main]` replaces `#[entry]` |
| `defmt`, `defmt-rtt`, `panic-probe` | Logs & panics, only readable with a probe (none here) |
| `embassy-usb` | The USB device stack and classes, see [USB.md](./USB.md) |
| `embassy-futures` | `join`, `select`: wait on several futures in one task |
| `static_cell` | `StaticCell`: `'static` values filled at runtime (below) |
| `portable-atomic` | Atomics for the M0+, needed by `static_cell` (below) |

#### Embassy and rp-hal Don't Mix

Both bring a boot2 and the interrupt vectors, so they can't be linked together.
The boot2 (`w25q080`, right for the RP2040-Zero flash) is in embassy-rp by
default, see [BUILD.md](./BUILD.md#the-boot).

<div align="center">
    <img src="../images/prawns.png">
</div>

### `init` and the Peripherals

```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
}
```

`init` sets the clocks, the PLLs and the timer. With rp-hal it took several
calls: `Watchdog::new`, `init_clocks_and_plls`, `Timer::new`, …

It returns `p`, every peripheral of the chip, **once**. Each field is a token
(`Peri<'d, T>`) that a driver takes by value:

```
  p ──┬── p.PIO0     ──> Pio::new(p.PIO0, Irqs)
      ├── p.DMA_CH0  ──> PioWs2812::…(…, p.DMA_CH0, …)
      ├── p.PIN_16   ──> PioWs2812::…(…, p.PIN_16, …)
      ├── p.USB      ──> (free)
      └── …
```

A peripheral moved into a driver can't be given to a second one: two drivers
on the same pin or DMA channel is a **compile error**.

<div align="center">
    <img src="../images/prawns.png">
</div>

### The Executor

#### `main` and the Tasks

`#[embassy_executor::main]` creates the executor, then runs `main` as its first
task. `main` sets up the hardware, spawns the other tasks, and can return: the
tasks keep running.

```rust
#[embassy_executor::task]
async fn test_led(mut led: Led) { loop { … } }

spawner.spawn(test_led(led).unwrap());
```

| Part | Role |
|---|---|
| `#[embassy_executor::task]` | Turns the `async fn` into a task, with a **static** slot for its future |
| `test_led(led)` | Puts the future in its slot, returns `Result<SpawnToken, _>` |
| `.unwrap()` | `Err` if the slot is taken (pool size 1 by default: spawned twice) |
| `spawner.spawn(token)` | Adds the task to the executor's run queue |

The future lives in a `static`, so everything it holds must be `'static` too:
the arguments are moved in (`led`), never borrowed from `main`.

#### Polling and Sleeping

A task runs until its next `.await` that isn't ready, then hands back to the
executor (`Pending`). With nothing ready, the executor sleeps with `WFE`. An
interrupt (timer, DMA, …) wakes the task's waker, which sets a flag and sends
`SEV`: the executor wakes up and polls that task again.

```
 time ───────────────────────────────────────────────────────────────►

 main      ██ spawn test_led, return
 test_led     ██ led.write ─ Pending        ██ … Timer ─ Pending
 executor           ░░░░░ WFE ░░░░░            ░░░░░░░ WFE ░░░░░░░
 interrupt                         ▲ DMA_IRQ_0
                                   └─ wake test_led
```

- **One core, one task at a time**: tasks share the CPU, each `.await` is a
  point where another task can run. A long computation without `.await`
  blocks all the others.
- **No busy loop**: with rp-hal the main loop spun and checked every timer.
  Here, the CPU sleeps until an interrupt has something for a task.

#### What a Task Is

An `async fn` that the executor runs **on its own**, next to the others. It's
started once with `spawn`, never called nor `.await`ed.

| | |
|---|---|
| Loops forever? | Often (`test_led`, `usb_run`), not always: `main` returns after its setup |
| Needs `.await`s | Yes: each one that waits gives the CPU to the other tasks |
| A `Timer` | Only one way to wait. A task can wait on the DMA, the USB, … |

<div align="center">
    <img src="../images/prawns.png">
</div>

### `'static` and `StaticCell`

#### Why `'static`

A task's future lives in a `static`, forever. So everything it holds must live
forever too: its type must be `'static`. A local variable of `main` doesn't
qualify, even if `main` never returns: the compiler doesn't know it.

```
 main's stack:  buf ◄── borrowed by device ──spawn──> task   ✗ not 'static
 static:        buf ◄── borrowed by device ──spawn──> task   ✓
```

#### `StaticCell`

A `static` lives forever, but writing into a `static mut` needs `unsafe`.
`StaticCell` (crate `static_cell`) is a `static` filled **once**, at runtime:

```rust
static BUF: StaticCell<[u8; 256]> = StaticCell::new(); // empty, in .bss

let buf: &'static mut [u8; 256] = BUF.init([0; 256]);  // filled, borrowed forever
```

| | |
|---|---|
| `StaticCell::new()` | `const`: the cell is reserved at link time, empty |
| `init(value)` | Moves `value` in, returns a `&'static mut` to it |
| `init` a second time | **Panics**: two `&mut` to the same memory can't exist |

It replaces the `static mut` + `unsafe` of the rp-hal code.

Several values can share one cell, in a struct. Borrowing **different fields**
with `&mut` at the same time is allowed, and each borrow stays `'static`:

```rust
let b = USB_BUFFERS.init(UsbBuffers { … });
Builder::new(…, &mut b.config_descriptor, &mut b.bos_descriptor, …);
```

#### `portable-atomic`

`init` must check "already filled?" and mark it filled **in one step**, or an
interrupt between the two could fill it twice. Most CPUs have an instruction
for that (compare-and-swap); the Cortex-M0+ doesn't.

| CPU | Atomic read / write | Compare-and-swap |
|---|---|---|
| Cortex-M3, M4, M33 | ✓ | ✓ (`LDREX` / `STREX`) |
| **Cortex-M0+** (RP2040) | ✓ | ✗ |

`static_cell` uses `portable-atomic`: the same atomics as `core`, with a
fallback when the instruction is missing.

```
 StaticCell::init ──> portable_atomic compare_exchange
                         ├── CPU has CAS ──> the instruction
                         └── M0+         ──> critical section: interrupts off,
                                             read + write, interrupts on
```

The fallback must be chosen in `Cargo.toml`, else the build fails
(*"requires atomic CAS but not available on this target"*):

```toml
portable-atomic = { version = "1", features = ["critical-section"] }
```

The critical section itself comes from `embassy-rp` (`critical-section-impl`).
We don't use `portable-atomic` directly: cargo merges the features, so the line
turns it on for `static_cell`'s copy.

<div align="center">
    <img src="../images/crab_back.png">
</div>

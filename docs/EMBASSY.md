<div align="center">
    <img src="../images/crab_coffee.png">
</div>

Notes from porting the firmware from rp-hal to embassy, one step at a time.

### Step 0: the project

#### Dependencies

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

The boot2 (`w25q080`, right for the RP2040-Zero flash) is in embassy-rp by default,
it goes in the `.boot2` section of `memory.x`.

rp-hal and embassy-rp can't be linked together: both bring a boot2 and the interrupt
vectors.

#### The smallest main

```rust
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_rp::init(Default::default());
}
```

`init` sets the clocks, the PLLs and the timer (with rp-hal: `Watchdog`,
`init_clocks_and_plls`, `Timer::new`, …). It returns `p`, all the peripherals.

#### Cargo profiles

| Command | Profile | Default |
|---|---|---|
| `cargo build` / `cargo run` | `dev` | `opt-level = 0` (no optimisation), `debug = true` |
| `cargo build --release` / `cargo run --release` | `release` | `opt-level = 3` (full optimisation), `debug = false` |

A `[profile.xxx]` section in `Cargo.toml` only changes the settings it lists.

`debug = 2` keeps the full debug info (machine code → source lines & variable names).
It stays in the ELF on the PC: `elf2uf2-rs` only copies the code and the data to the
`.uf2`, the firmware on the chip has the same size. Only useful with a probe
(`probe-rs`: stack traces, `defmt` locations, debugger), so not needed here.

**Prefer `--release`** (recommended, not required): async code works in a debug build,
but without optimisation:
- the firmware is several times bigger and slower (it still fits the RP2040-Zero)
- the futures (one state machine per `async fn`, in static memory) can take more RAM
- timing-sensitive code (matrix scan, UART) can miss its deadlines

```
cargo run --release
```

Or optimise the dev profile too, so a plain `cargo run` gives a decent build:

```toml
[profile.dev]
opt-level = "s"   # optimise for size
```

<div align="center">
    <img src="../images/crab_back.png">
</div>

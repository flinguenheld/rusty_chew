<div align="center">
    <img src="../images/crab_coffee.png">
</div>

How the firmware is built: what `.cargo/config.toml` and `memory.x` do.

### `Cargo.toml` vs `.cargo/config.toml`

| | `Cargo.toml` | `.cargo/config.toml` |
|---|---|---|
| Says | **What** to build (the package) | **How** cargo builds it (the tool) |
| Contains | Name, dependencies, features, `[[bin]]`, profiles | Target, linker, rustflags, runner, `target-dir` |
| Where | Root of the project | `.cargo/` in the project, a parent folder, or `~/.cargo/` |

A crate that depends on this one only needs its `Cargo.toml`. The `.cargo/config.toml` is about
this machine and this project.


<div align="center">
    <img src="../images/prawns.png">
</div>

### `.cargo/config.toml`

```toml
[target.'cfg(all(target_arch = "arm", target_os = "none"))']
runner = "elf2uf2-rs -d"
rustflags = [
  "-C", "linker=flip-link",
  "-C", "link-arg=--nmagic",
  "-C", "link-arg=-Tlink.x",
  "-C", "link-arg=-Tdefmt.x",
  "-C", "no-vectorize-loops",
]

[build]
target = "thumbv6m-none-eabi"
target-dir = "/goinfre/flinguen/rusty_chew-target"
```

#### `[target.'cfg(...)']`

The settings below it only apply when compiling for a bare-metal ARM chip (`target_arch = "arm"`,
`target_os = "none"`). A build for the PC (tests, tools) does not get them.

#### `runner`

What `cargo run` does with the ELF once it is built: `cargo run --release` calls
`elf2uf2-rs -d <path/to/elf>`.

| Runner | What it does |
|---|---|
| `elf2uf2-rs -d` | Converts the ELF to `.uf2`, copies it to the RP2040 in BOOTSEL mode (USB drive), `-d` = deploy |
| `probe-rs run --chip RP2040` | Flashes through a debug probe (SWD), then shows the `defmt` logs and the panics |

Without a probe the `defmt` logs go nowhere: `defmt-rtt` writes them into a RAM buffer that only a
probe reads.

#### `rustflags`

Flags given to `rustc` (`-C` = codegen option, `link-arg` = passed to the linker).

| Flag | Role |
|---|---|
| `linker=flip-link` | Wraps the linker: puts the stack **below** `.data`/`.bss`, so a stack overflow hits the start of RAM and faults (HardFault), instead of silently overwriting the variables |
| `--nmagic` | Don't align the sections on memory pages (4 KiB): no padding between them, the addresses of `memory.x` are kept |
| `-Tlink.x` | The linker script of `cortex-m-rt`: vector table, `.text`, `.data`, `.bss`, stack. It `INCLUDE`s `memory.x` |
| `-Tdefmt.x` | The linker script of `defmt`: the log strings go in a `.defmt` section that stays in the ELF, never on the chip (the chip only sends an index) |
| `no-vectorize-loops` | No loop vectorisation: the Cortex-M0+ has no SIMD, it only makes the code bigger |

The commented `-Z trap-unreachable=no` saves size but needs a nightly compiler.

#### `[build]`

| Key | Role |
|---|---|
| `target = "thumbv6m-none-eabi"` | The CPU to compile for, so no `--target` on each command |
| `target-dir` | Where the build files go (default: `target/` in the project) |

`thumbv6m-none-eabi`:

| Part | Meaning |
|---|---|
| `thumbv6m` | ARMv6-M, Thumb instructions: the Cortex-M0+ of the RP2040 |
| `none` | No operating system |
| `eabi` | The ARM calling convention (Embedded ABI) |

The ELF ends up in `<target-dir>/thumbv6m-none-eabi/release/mono`.

`target-dir` is out of `/home` (too small, 4.7 GB); `/goinfre` can be wiped, it's only a cache:
the next build is just longer.


<div align="center">
    <img src="../images/prawns.png">
</div>

### `memory.x`

```
MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;
```

The map of the memory, for the linker: where each part of the firmware goes. `link.x`
(`cortex-m-rt`) is generic for all Cortex-M, `memory.x` gives the addresses of this chip & board.

#### `MEMORY`

| Region | Address | Size | Content |
|---|---|---|---|
| `BOOT2` | `0x10000000` | 256 bytes | The second stage bootloader |
| `FLASH` | `0x10000100` | 2 MiB − 256 | The code & the constants (`.text`, `.rodata`), the initial values of `.data` |
| `RAM` | `0x20000000` | 256 KiB | Variables (`.data`, `.bss`), the stack, the embassy futures |

- `0x10000000` is the XIP flash: the external QSPI flash read as if it was memory (eXecute In Place).
- 2 MiB: the flash of the RP2040-Zero (W25Q16), other boards can have more.
- The RP2040 has 264 KiB of SRAM: 256 KiB striped (SRAM0-3) + 2 × 4 KiB (SRAM4, SRAM5 at
  `0x20040000`). Only the first 256 KiB are used here.

#### The boot

1. The bootrom (in the chip, read only) starts, copies the first 256 bytes of the flash in RAM,
   checks their CRC and runs them.
2. The boot2 configures the QSPI interface for this flash chip (W25Q080 compatible), enables the XIP.
3. It jumps to the vector table at `0x10000100`: `cortex-m-rt` copies `.data`, zeroes `.bss`, calls
   `main`.

Without a valid boot2 (wrong CRC) the RP2040 stays in BOOTSEL mode.

#### `EXTERN(BOOT2_FIRMWARE)` & `SECTIONS`

- `BOOT2_FIRMWARE` is a `static` of `embassy-rp` (feature `boot2-w25q080`, on by default) in the
  `.boot2` section. Nothing calls it, so the linker would remove it: `EXTERN` forces it to be kept.
- `SECTIONS` creates the output section `.boot2` at the start of `BOOT2`, `KEEP` stops the linker
  from removing it, `INSERT BEFORE .text` adds it to the sections of `link.x` instead of replacing
  them.

#### How the linker finds `memory.x`

There is no `build.rs`: the linker runs from the project root and finds `memory.x` there.
Cargo doesn't know about this file, so **after editing `memory.x`, nothing is rebuilt**:
`touch src/mono.rs` (or `cargo clean`) to force the link.

The embassy examples use a `build.rs` that copies `memory.x` in `OUT_DIR` and tells cargo to
rebuild when it changes:

```rust
println!("cargo:rerun-if-changed=memory.x");
```


<div align="center">
    <img src="../images/prawns.png">
</div>

### Writing in the flash at runtime

The firmware can erase & rewrite parts of its own flash, the data survives a power off
(settings, layer, recorded macros, …). The flash isn't RAM:

| Operation | Size | Time (≈) | Effect |
|---|---|---|---|
| Read | any | fast | like memory (XIP) |
| Erase | 4 KiB sector | ~45 ms | all bits to `1` (`0xFF`) |
| Write | 256 bytes page | ~1 ms | bits from `1` to `0` only |

To change a value: erase its whole sector, then write it again.

#### The catches

1. **The code runs from the same flash**: during an erase or a write, no code can be read from it.
   The writing code runs from RAM, the interrupts are paused, the second core is stopped:
   `embassy_rp::flash::Flash` does it.
2. **The keyboard freezes during an erase**: ~45 ms without scan nor USB. Fine for a rare save,
   not while typing.
3. **The flash wears out**: ~100 000 erases per sector. Write only when something changes, never
   on each key press. The `sequential-storage` crate spreads the writes over several sectors
   (wear levelling).
4. **Reserve the area in `memory.x`**, so the linker never puts code there.

#### Reserving the area

The data at the end of the flash, e.g. 16 KiB (4 sectors):

```
MEMORY {
    BOOT2   : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH   : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100 - 16K
    STORAGE : ORIGIN = 0x101FC000, LENGTH = 16K
}
```

`STORAGE` is not used by the linker, it only documents the area: what matters is the smaller
`FLASH`. If the firmware grows too much, the link fails instead of overwriting the data.

#### The driver

```rust
use embassy_rp::flash::{Blocking, Flash};

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const STORAGE_OFFSET: u32 = (FLASH_SIZE - 16 * 1024) as u32; // 0x1FC000

let mut flash = Flash::<_, Blocking, FLASH_SIZE>::new_blocking(p.FLASH);

flash.blocking_erase(STORAGE_OFFSET, STORAGE_OFFSET + 4096)?; // a whole sector
flash.blocking_write(STORAGE_OFFSET, &data)?;
flash.blocking_read(STORAGE_OFFSET, &mut buf)?;
```

- The offsets start at the beginning of the flash (`0x10000000`), not at `FLASH` of `memory.x`.
- The erase offsets must be aligned on 4 KiB.
- `FLASH_SIZE` must match the real flash (2 MiB on the RP2040-Zero).
- `Flash::new(p.FLASH, p.DMA_CH0)` (`Async`) reads with the DMA, the erase & write stay blocking.

<div align="center">
    <img src="../images/crab_back.png">
</div>

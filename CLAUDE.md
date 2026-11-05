# Rusty Chew

Firmware for the Chew keyboards (mono and split) on an RP2040-Zero, in Rust.

## Goal of the `embassy` branch

I'm porting the firmware from rp-hal to embassy **to learn embassy**. Working code
matters less than understanding it.

- The rp-hal version is the reference for the behaviour: the `main` branch, and the old
  files in `save/`.
- We go one step, one module at a time. The plan and the progress are in
  [docs/TODO.md](docs/TODO.md).
- **Files restart from scratch.** The old code is not adapted, only read for the
  behaviour. Each new file starts small: understand what is needed, get a first result
  (a LED color, a first text on the serial…), then grow it.
- **Open to new ideas and improvements.** Same behaviour is the baseline, not a limit:
  when embassy (async, tasks, channels…) allows a simpler or better design than the
  rp-hal one, propose it and explain the trade-offs.

## How we work

- **I write the code.** Don't create or edit source files unless I ask. Explain the
  concepts and show the code in the chat; I type it, then you review it.
- Explain **why**, not only what: which crate, which type, which interrupt, and what
  it replaces from rp-hal.
- Short steps: one idea at a time, then wait for me before going on.
- Point out mistakes and better ways, but leave the choice to me.
- Write a topic's doc in `docs/` once its step works and I understand it.
- When a step is done: check it off in `docs/TODO.md`. When a new interrupt line is
  bound: add it to the table in `docs/INTERRUPTS.md`.
- **Never commit.** When I ask, suggest a commit message in my style:
  `Topic - what changed` (e.g. `LED - create Led struct and its methods`).

## Embassy versions

The embassy API changes a lot between versions. Don't answer from memory: read the
source of the versions in `Cargo.lock`, in `~/.cargo/registry/src/*/` (e.g.
`embassy-rp-0.10.0/src/usb.rs`), and the examples of that version.

## Debugging

No probe: the firmware is flashed over USB with `elf2uf2-rs`, so `defmt` output goes
nowhere. Debug with the LED, and with the USB serial once it works. Don't suggest
`defmt::info!` or `probe-rs` to see something.
- **Correct my English** when I make mistakes (in my messages, comments and docs):
  give the corrected sentence briefly, at the start or end of your answer.

## Code style

New files follow `src/software/led.rs`:

- A struct per hardware part, which owns its driver (`Led` owns the `Ws2812`), with
  short methods. They chain when it makes the calls read well
  (`led.set(RED, 200).fade(300).await`).
- Sections with a banner, the name in capitals on the right:

  ```rust
  // ──────────                                ─────────────────────────────
  // ─────                               ───────────────────────── SET ─────
  ```

- A `///` doc comment on every function: what it does in the first line, then the
  why, the limits, and a small `text` table or graph when it helps.
- `//` comments explain the non-obvious lines (rounding, overflow, clamping).
- Tunable values (timings, sizes) go in `src/options.rs`, not in the module.
- Hardware variants are type aliases behind features (`#[cfg(feature = "zero")]`).
- Formatted with `rustfmt.toml` (79 columns).

## Documentation (`docs/`)

The docs are there to understand the project as it is, not to tell the story of the
port: no "step N" in them (the progress belongs in `docs/TODO.md`).

- One file per topic (`LED.md`, `INTERRUPTS.md`…). Firmware-wide concepts get their
  own file and are linked from the others, not explained inside one module's doc.
- Same style as the existing docs:
  - centered `crab_coffee.png` at the top, `crab_back.png` at the end, `prawns.png`
    between groups of sections (all in `../images/`)
  - `###` sections, `####` subsections
  - short sentences, tables for lists of facts (methods, pins, crates)
- **Lots of graphs**: layers, pipelines, timelines, state machines, memory layouts…
  - ASCII first
  - Mermaid only when ASCII gets too messy (many branches, sequence diagrams with many
    actors). Sequence diagrams use the compact init line (`mirrorActors: false`, small
    margins) and short actor names.

## Build

```
cargo build --release
cargo run --release   # flash with elf2uf2-rs, the board must be in boot mode
```

- Target `thumbv6m-none-eabi`, config in `.cargo/config.toml`, details in
  [docs/BUILD.md](docs/BUILD.md).

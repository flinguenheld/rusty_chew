# Rusty Chew

Firmware for the Chew keyboards (mono and split) on an RP2040-Zero, in Rust.

## Goal of the `embassy` branch

I'm porting the firmware from rp-hal to embassy **to learn embassy, embedded and
hardware**, from scratch. Understanding matters more than working code:

- Understand the concepts (I'm a beginner in hardware: link them to Petzold's *Code*
  when it helps)
- Discover new tools
- Have new ideas
- Write short and clean code, in small blocks
- Write the documentation

How:

- The rp-hal version (`main` branch, `save/`) is only read for the behaviour, never
  copied. Propose better designs with their trade-offs.
- One step at a time, plan and progress in [docs/TODO.md](docs/TODO.md). Each new file
  starts small (an LED color, a first text on the serial…), then grows.

## How we work

- **I write the code.** Don't edit source files unless I ask. Give me the problem, the
  concept and where to look (crate source, docs, examples), not a finished solution.
  Small API snippets are fine. Then review what I wrote.
- One idea per answer, then wait for me. Explain **why**: which crate, type, interrupt,
  and what it replaces from rp-hal.
- Point out mistakes and better ways, but leave the choice to me.
- When a step works and I understand it: write its doc in `docs/`, check it off in
  `docs/TODO.md`. New interrupt line bound: add it to `docs/INTERRUPTS.md`.
- **Never commit.** When I ask, suggest a message in my style: `Topic - what changed`
  (e.g. `LED - create Led struct and its methods`).
- **Correct my English** (messages, comments, docs) briefly, at the start or end of your
  answer.

## Embassy versions

The API changes a lot between versions. Don't answer from memory: read the source of the
versions in `Cargo.lock` in `~/.cargo/registry/src/*/` (e.g.
`embassy-rp-0.10.0/src/usb.rs`) and their examples.

## Debugging

No probe (flashed over USB with `elf2uf2-rs`): `defmt` output goes nowhere. Debug with
the LED and the USB serial, never `defmt::info!` or `probe-rs`.

## Code style

New files follow `src/software/led.rs`:

- A struct per hardware part, owning its driver (`Led` owns the `Ws2812`), with short
  methods that chain when it reads well (`led.set(RED, 200).fade(300).await`).
- Sections with a banner, the name in capitals on the right:

  ```rust
  // ──────────                                ─────────────────────────────
  // ─────                               ───────────────────────── SET ─────
  ```

- `///` on every function: what in the first line, then why, limits, and a small `text`
  table or graph when it helps. `//` for the non-obvious lines (rounding, overflow…).
- Tunable values in `src/options.rs`. Hardware variants as type aliases behind features
  (`#[cfg(feature = "zero")]`).
- `rustfmt.toml` (79 columns).

## Documentation (`docs/`)

The docs explain the project as it is, not the story of the port (no "step N").

- One file per topic (`LED.md`, `INTERRUPTS.md`…). Firmware-wide concepts get their own
  file, linked from the others.
- After writing or changing a doc: check its links, that it doesn't repeat another doc
  (link instead), and that it's listed in [docs/README.md](docs/README.md).
- When a step is done, review all the docs (duplicates, old facts, files to merge or
  split): report first, I choose.
- Style: centered `crab_coffee.png` at the top, `crab_back.png` at the end, `prawns.png`
  between groups of sections (in `../images/`). `###` sections, `####` subsections.
  Short sentences, tables for lists of facts.
- **Lots of graphs** (layers, pipelines, timelines, state machines, memory layouts),
  ASCII first. Mermaid only when ASCII gets messy; sequence diagrams use the compact
  init line (`mirrorActors: false`, small margins) and short actor names.

## Build

```
cargo build --release
cargo run --release   # flash with elf2uf2-rs, the board must be in boot mode
```

Target `thumbv6m-none-eabi`, details in [docs/BUILD.md](docs/BUILD.md).

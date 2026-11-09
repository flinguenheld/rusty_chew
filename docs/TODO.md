The `embassy` branch:

- Switching the firmware from rp-hal to embassy, with the same behaviour (`rp-hal` is the reference)
- Discovering embassy step by step, one module at a time
- Writing one doc per topic in `docs/` once its step works (`LED.md`, `INTERRUPTS.md`…)

### Done

- [x] Step 0: dependencies, smallest `main` in `src/mono.rs`, cargo profiles
- [x] `target-dir` in `/goinfre` (`/home` too small)
- [x] [BUILD.md](./BUILD.md): `.cargo/config.toml`, `memory.x`, flash storage
- [x] README cleaned (flash + docs only)

### Before the next build

- [ ] `src/lib.rs`: comment `layouts`, `options`, `software` (they need crates removed from `Cargo.toml`)
- [ ] `Cargo.toml`: comment `[[bin]] split`, or always build with `--bin mono`
- [ ] Check that `cargo build --release --bin mono` passes

### Steps

- [x] 1. LED: WS2812 on GP16 with PIO (`PioWs2812`), `bind_interrupts!`, `Timer::after`
- [ ] 2. Serial: `embassy-usb` CDC ACM replaces `usbd-serial`, to debug without a probe (feature `serial`)
- [ ] 3. Buzzer: PWM, songs as an async task
- [ ] 4. Matrix (mono): GPIO `Input`/`Output`, scan loop, rows discharge
- [ ] 5. USB HID: keyboard, mouse, NKRO with `embassy-usb` (replaces `usbd-human-interface-device`), next to the serial
- [ ] 6. Chew logic back: `software/`, `layouts`, `options` (remove the rp-hal & usbd types)
- [ ] 7. Split: direct pins, half-duplex UART on PIO (replaces `pio-uart`), `split_mail`, vbus detection (Gemini)
- [ ] 8. Status: LED & buzzer show the state
- [ ] 9. Features `zero`, `master`, `slave`: check every combination builds

### Later

- [ ] `build.rs` that copies `memory.x` (rebuild when it changes)
- [ ] Save settings in flash (`embassy_rp::flash`, `sequential-storage`)
- [ ] README: features, layout, thanks back (from `git show main:README.md`)
- [ ] Docs back & updated: MATRIX, REPORTS, SPLIT_LINK, TIMING, STATUS

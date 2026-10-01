# Pokeviewer

Offline, battery-powered Pokémon-of-the-day e-paper display for exactly one
board, the Waveshare ESP32-S3-ePaper-1.54-EN V2. Rust `no_std` firmware, a
Linux USB CLI for adult RTC setup, and a content pack compiled into the image.

## Commands

Cargo is the only entry point. Repository automation belongs in `xtask`, not
in a Makefile or Justfile; the release packaging scripts in `scripts/` are the
only shell exception.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo xtask help                            # every repository task
cargo xtask golden-check target/visual-diff # exact-pixel daily-card check
cargo xtask firmware-build                  # needs the esp-1.95.0.0 toolchain
```

Host commands never select an embedded target, so host Clippy and tests skip
modules gated on `target_arch = "xtensa"`; only `firmware-build` compiles them.
Without the [pinned toolchain](docs/development/toolchain.md), report the
firmware build as unchecked. [CI](docs/development/ci.md) runs the rest.

## Layout

| Path | Owns |
| --- | --- |
| `crates/pokeviewer-core` | `no_std` domain: schedule, content pack, battery state, USB protocol codec, renderer. No board, clock, filesystem, or heap. |
| `crates/pokeviewer-firmware` | Board runtime. Ungated modules such as `wake.rs`, `application.rs`, and `failure.rs` decide; xtensa-only modules such as `board.rs`, `runtime.rs`, `sleep.rs`, and `panel.rs` perform effects. |
| `crates/pokeviewer-esp32s3-pad-hold` | The only `unsafe` code: one audited GPIO42 hold-register write. |
| `crates/pokeviewerctl` | Linux x86-64 USB CLI: RTC set/read, battery report, diagnostics. |
| `xtask` | Firmware builds and flashing, offline content build, rendering, goldens. |

Put each new decision in `pokeviewer-core` or an ungated firmware module with
host tests; keep the xtensa-only side a thin adapter that feeds hardware facts
in and applies the result. Host tests run on every PR, while a device-only
defect needs a board, a battery, and often a 07:00 wake to reproduce.

## Rules

- [The product contract](docs/product-contract.md) fixes v1 scope. Changing a
  locked decision needs an accepted ADR first. Accepted ADRs in
  [docs/decisions](docs/decisions/README.md) are superseded by a new ADR that
  links both ways, never edited to change their outcome.
- Fix lints and failing checks instead of relaxing them. Do not lower workspace
  lint levels or weaken golden, content-integrity, firmware-budget, or
  release-package checks; they encode reviewed behaviour.
- Run `cargo xtask golden-update` only for a reviewed visual change, and name
  the changed cards in the PR.
- `cargo xtask content-fetch` is the only network step. Never call it from a
  build, test, or CI; the committed pack must rebuild offline byte-for-byte.
- The device belongs to a child. Keep child or household details, MAC
  addresses, USB serial numbers, private paths, credentials, and raw device
  logs out of commits, fixtures, docs, and CI artifacts; use placeholders such
  as `/dev/ttyACM0`. See [privacy and evidence](docs/privacy-and-evidence.md).
- Pokémon names and sprites are not MIT-licensed. Read
  [the third-party notices](THIRD_PARTY_NOTICES.md) before changing content.
- Branch names take a conventional prefix (`feat/`, `fix/`, `docs/`, `ci/`,
  `refactor/`, `test/`, `chore/`); commits follow Conventional Commits, for
  example `fix(firmware): ...`. Commits and PR text carry no AI attribution.
- Device testing is optional and bounded: at most one check for changed
  hardware behaviour, as listed in
  [hardware validation](docs/hardware/validation.md).

## Deeper docs

- Wake, sleep, PWR, BOOT, and 07:00: [state machine](docs/hardware/wake-sleep-state-machine.md)
- Pins and power rails: [V2 board contract](docs/hardware/v2-board-contract.md)
- USB protocol: [protocol v1](docs/usb-protocol-v1.md)
- Renderer and goldens: [rendering](docs/development/rendering.md)
- Content pipeline: [content tooling](docs/development/content-tooling.md)
- Review and release: [CONTRIBUTING.md](CONTRIBUTING.md), [publishing](docs/development/publishing.md)

# Continuous integration

The `CI` workflow runs five independent jobs and one aggregate gate on every
pull request and every push to `main`.

| Job | Responsibility | Artifact |
| --- | --- | --- |
| `Host checks` | host quality and policy | `host-check-logs` |
| `Offline content integrity` | regenerate and compare committed pack/assets | `content-integrity` |
| `Protocol compatibility` | codec, firmware handler, and Linux CLI tests | none |
| `Visual and recovery goldens` | daily frames, recovery frames, deliberate diff | `visual-proof` |
| `ESP32-S3 release` | target builds, budgets, and two-build section hashes | `esp32s3-release` |
| `Release matrix` | requires every preceding job to succeed | none |

Host quality covers formatting, Clippy, tests, documentation warnings, a
locked build, dependency policy, and workflow syntax. Content and visual checks
are separate so their failures are independently identifiable. Target
validation installs pinned Xtensa Rust and performs locked release builds of
the application, sleep diagnostic, and USB provisioning images.

Host commands do not set or inherit an embedded default target. The firmware
job selects `xtensa-esp32s3-none-elf` through `cargo xtask firmware-build` and
the sleep-diagnostic and USB-provisioning build commands. Neither job runs the
content generator, so normal CI makes no PokéAPI request.

The visual job uses only the committed content pack, renderer, raw 5,000-byte
goldens, and manifest. If a frame changes, `visual-golden-diff` contains the
expected, actual, and exact XOR PNG plus a coordinate/hash report for every
changed case. It also regenerates every recovery PNG byte-for-byte and runs the
intentional one-pixel failure demonstration.

The firmware job fixes `SOURCE_DATE_EPOCH` to the revision timestamp, verifies
a nonzero entry point, enforces 200,000 linked text bytes, 16,384 linked data
bytes, and a 65,536-byte content-pack limit, then rebuilds from a cleaned target
and compares hashes of all load-bearing ELF sections. A one-byte text budget is
run as an expected-failure demonstration. Debug-only ELF sections are excluded
because they may contain build paths and are not flashed.

Third-party actions are pinned to full commit SHAs. The comments beside action
references record the reviewed release tag where one exists. Tool inputs also
pin Rust, `cargo-deny`, `actionlint`, and `espup` versions.

## Failure propagation

Every logged command enables Bash `pipefail` before piping output through
`tee`. A failing Cargo or policy command therefore fails its step rather than
being hidden by a successful log write.

The five work jobs deliberately do not depend on each other:

- a failing host unit test fails `Host checks` before its artifact upload,
  while the remaining jobs continue independently;
- a target compiler or linker error fails `ESP32-S3 release`, while host tests
  remain independently reportable;
- corrupt generated content or protocol incompatibility has its own named
  result; and
- failure-oriented artifact uploads use `if: always()` so available diagnostic logs survive
  a failed check.

`Release matrix` depends on all five and is the single release-grade gate.
Temporary known-failing commits are not retained in project history; deliberate
failure modes are exercised as successful assertions.

## Local equivalents

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny --locked check
actionlint
cargo xtask content-build
cargo xtask golden-check target/visual-diff
cargo xtask render-recovery-screens target/recovery-screens
cargo xtask firmware-build
cargo xtask sleep-diagnostic-build
cargo xtask usb-provisioning-build
scripts/check-firmware-artifact.sh \
  target/xtensa-esp32s3-none-elf/release/pokeviewer-firmware \
  target/firmware-proof
```

The final command requires the embedded setup in the
[toolchain guide](toolchain.md).

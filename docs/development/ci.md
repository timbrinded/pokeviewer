# Continuous integration

The `CI` workflow runs two independent jobs on every pull request and every
push to `main`.

| Job | Responsibility | Artifact |
| --- | --- | --- |
| `Host checks` | host quality and policy | `host-check-logs` |
| `ESP32-S3 release` | target build and size | `esp32s3-release` |

Host quality covers formatting, exact visual-golden comparison, Clippy, tests,
a locked build, dependency policy, and workflow syntax. Target validation
installs pinned Xtensa Rust, performs locked release builds of the application
and sleep diagnostic, and reports the linked application-firmware size.

Host commands do not set or inherit an embedded default target. The firmware
job selects `xtensa-esp32s3-none-elf` through `cargo xtask firmware-build` and
`cargo xtask sleep-diagnostic-build`. Neither job runs the content generator,
so normal CI makes no PokéAPI request.

The visual check uses only the committed content pack, renderer, raw 5,000-byte
goldens, and manifest. If a frame changes, `visual-golden-diff` contains the
expected, actual, and exact XOR PNG plus a coordinate/hash report for every
changed case.

Third-party actions are pinned to full commit SHAs. The comments beside action
references record the reviewed release tag where one exists. Tool inputs also
pin Rust, `cargo-deny`, `actionlint`, and `espup` versions.

## Failure propagation

Every logged command enables Bash `pipefail` before piping output through
`tee`. A failing Cargo or policy command therefore fails its step rather than
being hidden by a successful log write.

The jobs deliberately do not depend on each other:

- a failing host unit test fails `Host checks` before its artifact upload,
  while `ESP32-S3 release` continues independently;
- a target compiler or linker error fails `ESP32-S3 release`, while host tests
  remain independently reportable; and
- both artifact uploads use `if: always()` so available diagnostic logs survive
  a failed check.

This job separation is the reviewable proof of failure routing. Temporary
known-failing commits are not retained in project history.

## Local equivalents

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --all-targets --locked
cargo deny --locked check
actionlint
cargo xtask golden-check target/visual-diff
cargo xtask firmware-build
cargo xtask sleep-diagnostic-build
```

The final command requires the embedded setup in the
[toolchain guide](toolchain.md).

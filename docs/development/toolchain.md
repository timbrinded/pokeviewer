# Toolchain and source builds

Host commands use stable Rust. Only the ESP32-S3 build uses Espressif's Xtensa
Rust toolchain, selected by `cargo xtask`. There is no workspace-wide embedded
target, so `cargo test --workspace` stays a host command.

## Pinned versions

| Component | Version |
| --- | --- |
| Host Rust | 1.96.0 (`rust-toolchain.toml`) |
| Espressif Xtensa Rust | 1.95.0.0, toolchain name `esp-1.95.0.0` |
| `espup` | 0.17.1 |
| `espflash` | 4.5.0 |
| `esp-hal` | Git revision `434755e0447fc1a4ba30fd84da3cf746ec082e00` (version 1.1.0); see [ADR 0004](../decisions/0004-use-esp-idf-aligned-rtc-deep-sleep.md) |
| `epd-waveshare` | 0.6.0 |
| `embedded-graphics` | 0.8.2 |
| `pcf85063a` | 0.1.1 |

Runtime crates are exact-pinned in the crate manifests and resolved in
`Cargo.lock`. The panel driver is the `epd1in54_v2` module of
`epd-waveshare`. With default features disabled, `epd-waveshare` 0.6.0 does not
compile unless `epd2in13_v2` or `epd2in13_v3` is enabled, so the firmware
enables `epd2in13_v3` without using that driver.

## Install

```sh
cargo install espup --version 0.17.1 --locked
cargo install espflash --version 4.5.0 --locked
espup install \
  --name esp-1.95.0.0 \
  --targets esp32s3 \
  --toolchain-version 1.95.0.0
```

Load the environment file that `espup` prints, then confirm the tools:

```sh
rustup run esp-1.95.0.0 rustc --version
espflash --version
```

On Linux, add your account to the serial-device group (`uucp` on Arch,
`dialout` on Debian and Ubuntu) and sign in again. Do not use `sudo` or a
world-writable device node.

## Build and flash from source

```sh
cargo xtask firmware-build
cargo xtask firmware-flash
cargo build --release --locked -p pokeviewerctl
```

`firmware-flash` builds the release firmware and runs
`espflash flash --monitor --chip esp32s3` through the Cargo runner. Exit the
monitor with `Ctrl+C`. Start download mode first, with the battery
disconnected, as in the [README quick start](../../README.md#3-start-download-mode).
Then set the time with `target/release/pokeviewerctl` as in step 6.

`cargo xtask help` lists the diagnostic build and flash commands. The
[hardware validation guide](../hardware/validation.md) describes each image.

The target configuration in `.cargo/config.toml` links `linkall.x`. Without
it, the linker can produce an ELF with an undefined `_start` and no
application code. `scripts/check-firmware-artifact.sh` rejects an image whose
entry point is zero.

## Dependency boundary

The firmware excludes Wi-Fi, Bluetooth, an RTOS, an allocator, audio, SD-card
support, and the Embassy executor. Board-facing code stays behind project
adapters so HAL details do not reach content, scheduling, or rendering code.
RTC access goes through the project-owned `Rtc` trait around `pcf85063a`.

References:

- [Espressif Rust toolchain installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
- [Xtensa Rust 1.95.0.0 release](https://github.com/esp-rs/rust-build/releases/tag/v1.95.0.0)
- [`epd-waveshare` 0.6.0](https://docs.rs/epd-waveshare/0.6.0/epd_waveshare/epd1in54_v2/)
- [`embedded-graphics` 0.8.2](https://docs.rs/embedded-graphics/0.8.2/embedded_graphics/)
- [`pcf85063a` 0.1.1](https://docs.rs/pcf85063a/0.1.1/pcf85063a/)

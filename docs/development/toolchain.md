# ESP32-S3 toolchain

Pokeviewer keeps ordinary host commands on stable Rust while pinning the
ESP32-S3 build to Espressif's Xtensa Rust toolchain. There is deliberately no
workspace-global embedded target: `cargo test --workspace` remains a host
command.

## Pinned versions

| Component | Version |
| --- | --- |
| Host Rust | 1.96.0 |
| Espressif Xtensa Rust | 1.95.0.0 |
| `espup` | 0.17.1 |
| `espflash` | 4.5.0 |
| `esp-hal` | 1.1.1 |
| `epd-waveshare` | 0.6.0 |
| `embedded-graphics` | 0.8.2 |
| `pcf85063a` | 0.1.1 |

Runtime crates are exact-pinned in `Cargo.toml` and resolved in `Cargo.lock`.
The 1.54-inch V2 driver is the crate's `epd1in54_v2` module, not a Cargo
feature. `epd-waveshare` 0.6.0 also requires either its `epd2in13_v2` or
`epd2in13_v3` feature to compile when defaults are disabled, so the manifest
enables `epd2in13_v3` as an upstream compatibility workaround. Pokeviewer does
not instantiate that panel driver.

## Install

Install the host tools:

```sh
cargo install espup --version 0.17.1 --locked
cargo install espflash --version 4.5.0 --locked
```

Install the named Xtensa toolchain:

```sh
espup install \
  --name esp-1.95.0.0 \
  --targets esp32s3 \
  --toolchain-version 1.95.0.0
```

Follow the environment instructions printed by `espup`. Confirm the tools:

```sh
rustup run esp-1.95.0.0 rustc --version
espflash --version
```

## Check and build

Run host validation with the repository default toolchain:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Build the release firmware through the repository task:

```sh
cargo xtask firmware-build
```

The equivalent low-level command is:

```sh
cargo +esp-1.95.0.0 build \
  --package pokeviewer-firmware \
  --bin pokeviewer-firmware \
  --target xtensa-esp32s3-none-elf \
  --locked \
  --release
```

## Flash and monitor

Connect the board by USB, then run:

```sh
cargo xtask firmware-flash
```

The configured runner invokes `espflash flash --monitor --chip esp32s3`.
Exit the monitor with `Ctrl+C`.

Linux users must have permission to open the device node, commonly by belonging
to the distribution's serial-device group and starting a new login session.
Do not add device serial numbers, home-directory paths, or raw probe logs to the
repository.

## Dependency boundary

The firmware intentionally excludes Wi-Fi, Bluetooth, an RTOS, an allocator,
audio, SD-card support, and a full Embassy executor. Board-facing APIs belong
behind project adapters so unstable HAL details do not leak into content,
scheduling, or rendering code.

The board contract fixes SPI2 and the 1.54-inch V2 panel; code should import
`epd_waveshare::epd1in54_v2`. RTC access must go through a project-owned
abstraction around `pcf85063a`.

Primary references:

- [Espressif Rust installation](https://docs.espressif.com/projects/rust/book/installation/riscv-and-xtensa.html)
- [Xtensa Rust 1.95.0.0 release](https://github.com/esp-rs/rust-build/releases/tag/v1.95.0.0)
- [`esp-hal` 1.1.1](https://docs.rs/esp-hal/1.1.1/esp_hal/)
- [`epd-waveshare` 0.6.0](https://docs.rs/epd-waveshare/0.6.0/epd_waveshare/epd1in54_v2/)
- [`embedded-graphics` 0.8.2](https://docs.rs/embedded-graphics/0.8.2/embedded_graphics/)
- [`pcf85063a` 0.1.1](https://docs.rs/pcf85063a/0.1.1/pcf85063a/)

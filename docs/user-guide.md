# Adult setup and operation guide

Pokeviewer supports exactly the non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. It is a development-board project, not a
certified finished children's toy. No battery is included.

## Linux prerequisites

For a source installation, use the pinned host and Xtensa setup in the
[toolchain guide](development/toolchain.md), a data-capable USB cable, and a
Linux account with normal read/write permission for the Espressif serial
device. Add the account to the distribution's appropriate device group and
re-login; never use a world-writable device-node workaround.

Before connecting a cell, read the [safety guide](safety.md).

## Install a release bundle

Download `pokeviewer-v1.1.0.tar.gz` and its adjacent
`pokeviewer-v1.1.0.tar.gz.sha256` from the official GitHub release. Verify and
extract the archive:

```console
sha256sum --check pokeviewer-v1.1.0.tar.gz.sha256
tar -xzf pokeviewer-v1.1.0.tar.gz
cd pokeviewer-v1.1.0
sha256sum --check SHA256SUMS
```

Read the bundled `SAFETY.md`, then follow the bundled `FLASHING.md`. That guide
uses the merged V2 firmware image and Linux x86-64 `pokeviewerctl` binary from
the same verified archive. Do not substitute a binary copied from an issue,
chat, third-party mirror, or source-tree build.

## Install from a source checkout

From a clean, verified release checkout:

```console
export RUSTUP_HOME="$PWD/.rustup-cache"
source .tools/export-esp.sh
cargo xtask firmware-build
cargo xtask firmware-flash
cargo build --release --locked -p pokeviewerctl
```

Keep USB attached for first-time RTC setup. The setup screen reads:

```text
SET TIME
CONNECT USB
RTC RUN
POKEVIEWERCTL
```

Use an explicit device path locally; do not paste it into public evidence:

```console
target/release/pokeviewerctl info --device DEVICE
target/release/pokeviewerctl set-rtc \
  --device DEVICE --datetime YYYY-MM-DDTHH:MM:SS
target/release/pokeviewerctl set-rtc \
  --device DEVICE --now
target/release/pokeviewerctl get-rtc --device DEVICE
target/release/pokeviewerctl diagnostics --device DEVICE
```

The set command validates all calendar fields and returns the RTC read-back.
Firmware then restarts and renders the correct card.

## Parent session

Normal PWR taps do not change the display. To change the time with the battery
installed:

1. Connect a data-capable USB cable.
2. Start `pokeviewerctl set-rtc --device DEVICE --now --wait-for-device`.
3. Press and hold `PWR` for three seconds.
4. Release `PWR` when `SET TIME` appears.
5. Wait for the RTC read-back.

The firmware requires a continuous three-second hold and a valid USB protocol
frame. It then keeps the parent session open for two minutes. The device path
wait is 60 seconds. The command allows time for the `SET TIME` screen to
refresh before it sends the time. A PWR hold without an active command does
not change the screen. The `BOOT` button is service-only.

To enter storage mode, start this command before the PWR hold:

```console
target/release/pokeviewerctl enter-storage \
  --device DEVICE \
  --confirm-time-loss \
  --wait-for-device
```

Storage mode shows `SET TIME`, clears the RTC, drops the board power latch,
and configures no ESP wake source. A later `PWR` press starts the device and
requires time setup.

## Normal operation

At 07:00 local time the device wakes, displays the weekday, Pokémon Yellow
sprite, English name, and canonical type or types, then returns to deep sleep.
The complete e-paper card remains visible without panel power.

The top corner shows an approximate battery value in 10 percent steps. The
value comes from a generic LiPo open-circuit-voltage curve. It is not a fuel
gauge. Below the low threshold, a lightning icon and `CHARGE!` appear. The
warning clears at the higher hysteresis threshold. An implausible ADC sample
shows `?%`. Firmware does not use this estimate as a safety control.

The board has no dedicated USB-power sense input. A power-only USB charger can
look like a full battery to the voltage input. USB operation with no battery
can therefore show `100%`. Use the percentage only as a coarse battery-mode
estimate.

Before 07:00 the prior display day intentionally remains. There is no clock,
touchscreen, child-facing button flow, Wi-Fi, BLE, SD-card dependency, runtime
API, account, location service, or internet requirement. All 151 entries and
the deterministic schedule are compiled into the firmware.

If all power is exhausted, the RTC may report oscillator loss. The retained
e-paper image can still look plausible, but firmware will not trust it or show
a new daily card. Recharge under adult supervision, connect USB, and set the
RTC again.

Battery runtime is not guaranteed. The firmware estimate does not replace
battery protection, charger behavior, or product qualification.

## Recovery and maintenance

Follow the [troubleshooting guide](troubleshooting.md) for every displayed
code. Firmware updates require an adult and USB; the device has no over-the-air
update path. For a clean reinstall, verify the release again, disconnect the
battery, flash over USB, reconnect the battery, set/read the RTC, and confirm
that the daily card is correct.

Public photographs and logs must follow the
[privacy rules](privacy-and-evidence.md).

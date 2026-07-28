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

Before connecting a cell, complete the
[battery polarity gate](hardware/v2-board-contract.md#battery-connector-safety-gate)
and read the [safety guide](safety.md).

## Build and flash

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
target/release/pokeviewerctl get-rtc --device DEVICE
target/release/pokeviewerctl diagnostics --device DEVICE
```

The set command validates all calendar fields and returns the RTC read-back.
Firmware then restarts and renders the correct card.

## Normal operation

At 07:00 local time the device wakes, displays the weekday, Pokémon Yellow
sprite, English name, and canonical type or types, then returns to deep sleep.
The complete e-paper card remains visible without panel power.

Before 07:00 the prior display day intentionally remains. There is no clock,
touchscreen, child-facing button flow, Wi-Fi, BLE, SD-card dependency, runtime
API, account, location service, or internet requirement. All 151 entries and
the deterministic schedule are compiled into the firmware.

If all power is exhausted, the RTC may report oscillator loss. The retained
e-paper image can still look plausible, but firmware will not trust it or show
a new daily card. Recharge under adult supervision, connect USB, and set the
RTC again.

Battery runtime is not guaranteed. Measure the exact assembled device and use
the [capacity calculator](hardware/battery-sizing.md) for the intended
protected cell.

## Recovery and maintenance

Follow the [troubleshooting guide](troubleshooting.md) for every displayed
code. Firmware updates require an adult and USB; the device has no over-the-air
update path. For a clean reinstall, verify the release again, disconnect the
battery, flash over USB, reconnect only after polarity inspection, set/read the
RTC, and confirm the displayed framebuffer against the release evidence.

Public photographs and logs must follow the
[privacy rules](privacy-and-evidence.md).

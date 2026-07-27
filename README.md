# Pokeviewer

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 board.

The project is at the start of its v1 implementation. The authoritative scope
is the [v1 product contract](docs/product-contract.md); work is tracked under
the [v1.0.0 delivery issue](https://github.com/timbrinded/pokeviewer/issues/1).
Firmware supports only the
[documented V2 board contract](docs/hardware/v2-board-contract.md).
Panel diagnostics follow the
[e-paper bring-up procedure](docs/hardware/panel-bring-up.md), and RTC
qualification follows the
[PCF85063 bring-up procedure](docs/hardware/rtc-bring-up.md). The separate
[deep-sleep qualification](docs/hardware/deep-sleep-qualification.md) covers
RTC wake behavior and battery measurements.

## V1 experience

Once a day at 07:00 local time, the device:

1. wakes from deep sleep;
2. selects the next entry from a fixed 151-day Generation I rotation;
3. displays the weekday, Pokémon Yellow sprite, English name, and current
   canonical type or types; and
4. returns to deep sleep while the e-paper retains the card.

There is no child-facing interaction and no runtime network access.

## Important notices

Pokeviewer is an unofficial, non-commercial fan project. It is not affiliated
with, endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc.,
or The Pokémon Company International.

The project's MIT license will cover original source code only. Pokémon names,
characters, artwork, sprites, and related media are third-party property and
are excluded from that license. See [Third-party notices](THIRD_PARTY_NOTICES.md)
before redistributing a build or asset pack.

Public project evidence must follow the
[privacy and evidence rules](docs/privacy-and-evidence.md).

## Decisions

Significant decisions are recorded in the
[architecture decision log](docs/decisions/README.md). The accepted
[content-pack and daily-schedule contract](docs/content-pack-v1.md) defines the
offline wire format and deterministic 151-day rotation.
The reviewed inputs, generated pack, validation report, and sprite contact
sheet are indexed in the [content directory](content/README.md).

## Development

The repository is a Cargo workspace:

| Package | Responsibility |
| --- | --- |
| `pokeviewer-core` | deterministic `no_std` domain and rendering logic |
| `pokeviewer-firmware` | supported-board hardware integration |
| `pokeviewerctl` | Linux USB provisioning and diagnostics |
| `xtask` | repository-local automation |

From the repository root:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo xtask help
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and review rules.
Embedded setup, builds, flashing, and monitoring are documented in the
[toolchain guide](docs/development/toolchain.md). See the
[CI guide](docs/development/ci.md) for automated checks and artifacts. Explicit
cache refresh and offline pack conversion are documented in the
[content tooling guide](docs/development/content-tooling.md).
The [shared-renderer guide](docs/development/rendering.md) documents the exact
panel buffer, host evidence command, and renderer memory budget.
The selected [v1 daily-card design](docs/design/daily-card-v1.md) fixes the
four-element visual hierarchy and indexes the complete 151-card review sheet.
Exact framebuffer regressions are covered by the
[visual-golden workflow](docs/development/visual-testing.md).
Wired RTC provisioning uses the versioned
[USB protocol](docs/usb-protocol-v1.md) and Linux `pokeviewerctl` utility.
Invalid clocks are governed by the
[RTC setup and recovery contract](docs/rtc-recovery.md).
The normal V2 firmware composition and current memory budget are documented in
[on-device integration](docs/development/on-device-integration.md).
Battery operation follows the
[07:00 wake-refresh-sleep state machine](docs/hardware/wake-sleep-state-machine.md);
capacity must be calculated from [measured current](docs/hardware/battery-sizing.md).
Expected faults follow the
[bounded failure and recovery contract](docs/hardware/failure-recovery.md).
Hardware releases must pass the
[V2 qualification procedure](docs/hardware/release-qualification.md).
The unattended acceptance run follows the
[seven-day physical protocol](docs/hardware/seven-day-run.md).

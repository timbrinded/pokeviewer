---
status: accepted
date: 2026-10-01
decision-makers:
  - Project maintainer
---

# Use BOOT as an adult restart and GPIO3 as a status light

## Context and Problem Statement

The V2 board has two buttons: `PWR` on GPIO18 and `BOOT` on GPIO0. It has no
chip-reset button. With the battery connected, the ESP32-S3 never loses
power, so an adult cannot reset it without opening the enclosure and
disconnecting the battery.

[ADR 0005](0005-use-no-wake-deep-sleep-for-terminal-failures.md) made terminal
failures enter deep sleep with no wake source. The recovery screens then ask
for a `RESET` that the assembled device cannot perform.

The three-second `PWR` hold has no visible acknowledgement. The e-paper does
not change until a computer sends a protocol frame, so an adult cannot tell
whether the hold was recognized.

The board's green LED on GPIO3 is unused. Its anode connects to 3V3 through
24 kΩ, so driving GPIO3 low turns it on. The orange LED is driven by the
ETA6098 charger `STAT` output and is not under firmware control.

## Decision Drivers

- An adult must be able to restart the firmware without opening the device.
- A child's short press must not change the display.
- `BOOT` must keep its ROM download-mode function at power-up.
- Terminal failures must not create automatic retry loops.
- Button holds that start adult work need visible acknowledgement.
- A wake line that remains low must not be armed for EXT1 `ANY_LOW` sleep.

## Considered Options

- Keep `BOOT` service-only and keep no-wake terminal sleep.
- Arm `BOOT` as a one-second-hold restart wake in every EXT1 sleep, and use
  the green LED for adult-session feedback.
- Add a `PWR` long-press restart or power-off.

## Decision Outcome

Chosen option: "Arm `BOOT` as a one-second-hold restart wake in every EXT1
sleep, and use the green LED for adult-session feedback."

Release firmware arms GPIO0 in daily, setup, and terminal-failure sleep. A
`BOOT` wake that stays held for one second flashes the green LED once and runs
the normal reset path: it reads the RTC and refreshes the card, `SET TIME`, or
recovery screen. It does not sample the battery. A shorter press returns to
sleep without changing the display. An alarm refresh or `PWR` parent session
in the same wake takes precedence.

Terminal failures arm only GPIO0. A restart runs the failed path once more and
returns to the same terminal state if the failure persists.

The green LED is on while the device listens for or serves a computer: after a
recognized three-second `PWR` hold, and during invalid-RTC USB setup. It turns
off before the device sleeps or restarts.

Before each sleep, firmware waits up to ten seconds for every requested wake
line to rise. A line that stays low is handled by `select_sleep_wake_sources`
rather than an unbounded awake loop.

### Consequences

- Good, because an adult can recover from every terminal failure without
  opening the enclosure.
- Good, because the `PWR` hold and `BOOT` restart have visible acknowledgement.
- Good, because the existing EXT1 boundary and external GPIO0 pull-up add no
  sleep current.
- Bad, because a held `BOOT` press restarts the firmware and refreshes the
  panel. The one-second threshold reduces, but does not remove, accidental
  restarts.
- Bad, because terminal sleep is no longer strictly wake-free.

### Confirmation

Host tests cover `BOOT` wake classification and the precedence of alarm and
parent work. A device check must show one green flash and one refresh after a
one-second `BOOT` hold, no change after a short press, a lit LED during a
`PWR` session, and recovery from one injected terminal failure.

## More Information

- Supersedes the no-wake outcome of
  [ADR 0005](0005-use-no-wake-deep-sleep-for-terminal-failures.md). Its single
  bounded display attempt and no-automatic-retry rules remain active.
- Amends the `BOOT` rule in the [product contract](../product-contract.md).
- [Wake, parent-session, and 07:00 state machine](../hardware/wake-sleep-state-machine.md)
- [V2 board contract](../hardware/v2-board-contract.md)

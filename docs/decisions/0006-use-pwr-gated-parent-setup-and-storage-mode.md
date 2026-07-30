---
status: accepted
date: 2026-07-29
decision-makers:
  - Project maintainer
---

# Use PWR-gated parent setup and storage mode

## Context and Problem Statement

The v1.0.0 firmware accepts RTC setup only after an invalid clock causes an
unbounded USB service loop. A shipped device keeps its battery connected, so a
parent needs a bounded way to set the clock without opening the enclosure or
flashing firmware. The same hardware also needs an explicit factory storage
state that leaves the first-use screen visible and removes battery power.

The exposed PWR button must not become a child-facing control. The BOOT button
must remain a flashing control.

Should setup remain tied to invalid RTC state, start whenever USB power is
present, or require a deliberate PWR hold and a valid USB protocol frame?

## Decision Drivers

- A normal PWR tap must not change the retained display.
- A charger or power-only cable must not start parent setup.
- Parent setup must have a fixed maximum duration.
- Time setting must not require battery disconnection or reflashing.
- Factory storage must require an explicit destructive command.
- Hardware behavior must follow the Waveshare ESP-IDF example and pinned
  ESP-HAL implementation.

## Considered Options

- Keep setup available only while the RTC is invalid.
- Start setup whenever USB power is present.
- Require a continuous PWR hold and a valid versioned USB frame.

## Decision Outcome

Chosen option: "Require a continuous PWR hold and a valid versioned USB
frame."

Production deep sleep uses ESP32-S3 EXT1 `ANY_LOW` with GPIO5 for the PCF85063
alarm and GPIO18 for PWR. Firmware snapshots the EXT1 status before it
configures the next sleep and combines that status with the RTC alarm flag.

For a valid RTC, GPIO18 wake starts a 50 ms debounce. A continuous three-second
hold opens a 15-second frame gate. The first valid protocol frame proves that a
data host is present and starts a two-minute parent session. A tap, an early
release, a power-only cable, malformed traffic, or a timeout preserves the
display and returns to sleep.

An invalid RTC renders `SET TIME`, serves USB for two minutes, and then enters
PWR-only deep sleep.

The parent session can set and read back the RTC. It can also accept an
explicit storage command. Storage mode refreshes `SET TIME`, acknowledges the
command, software-resets the PCF85063, confirms oscillator-stop state, drives
GPIO17 low, and enters deep sleep without a wake source. Removing USB then
removes board power. The next PWR start returns to first-use setup.

### Consequences

- Good, because PWR remains inert for normal child interaction.
- Good, because USB data, not USB power, proves parent intent.
- Good, because every active USB interval is bounded.
- Good, because parents can correct time without opening the enclosure.
- Bad, because setup requires a Linux host and the released CLI.
- Bad, because storage mode deliberately loses RTC state.
- Bad, because EXT1 wake classification adds retained-pad and status-register
  handling to the sleep boundary.

### Confirmation

Host tests must cover every wake classification, RTC read-back, storage
authorization, and deferred storage action. The target build fixes the hold,
frame-gate, and session limits. Device tests must prove those physical timing
boundaries.

One bounded V2-board test must prove GPIO18 EXT1 wake and status, release before
resleep, and a later GPIO5 alarm wake. A separate factory-flow test must prove
the first-use screen, successful storage response, power removal after USB
disconnect, and first-use setup after the next PWR start.

## Pros and Cons of the Options

### Keep invalid-RTC-only setup

- Good, because it requires no new wake source.
- Bad, because a parent cannot correct a valid but incorrect clock.

### Start setup from USB power

- Good, because the parent does not need a button sequence.
- Bad, because a charger and a data host are indistinguishable at power-on.

### Require PWR hold and a valid frame

- Good, because it combines physical and protocol intent.
- Good, because normal taps and power-only cables have no visible effect.
- Bad, because the runtime must manage a bounded parent-session state machine.

## More Information

- [ADR 0004](0004-use-esp-idf-aligned-rtc-deep-sleep.md)
- [ADR 0005](0005-use-no-wake-deep-sleep-for-terminal-failures.md)
- This decision supersedes their RTC-only wake and unbounded invalid-RTC setup
  details. Their ESP-IDF sleep-entry and no-wake failure decisions remain
  active.
- [USB protocol v1](../usb-protocol-v1.md)
- [Wake and sleep state machine](../hardware/wake-sleep-state-machine.md)
- [Waveshare V2 sleep example](https://github.com/waveshareteam/ESP32-S3-ePaper-1.54/tree/3f96beedd2e8daa35996abd0c055a7d394336dfb/02_Example/ESP-IDF/V2/12_RTC_Sleep_Test)

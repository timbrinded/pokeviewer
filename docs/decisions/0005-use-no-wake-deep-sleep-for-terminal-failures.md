---
status: accepted
date: 2026-07-28
decision-makers:
  - Project maintainer
---

# Use no-wake deep sleep for terminal failures

## Context and Problem Statement

ADR 0004 kept classified terminal failures awake so their logs remained
observable while normal RTC-alarm deep sleep was being qualified. The normal
sleep boundary now passes, so an awake terminal would turn a bounded software
failure into an unbounded battery drain.

Invalid RTC state is different when its bus remains available: the device must
stay awake long enough to accept the bounded wired provisioning protocol. Pack,
panel, alarm, and unexpected-wake failures require adult intervention and have
no useful automatic recovery.

Should those non-provisioning failures remain awake, retry after a timed sleep,
turn the board power latch off, or enter deep sleep without a wake source?

## Decision Drivers

- A classified failure must not create an unbounded active-power state.
- Failure handling must never retry, refresh, or reboot itself indefinitely.
- The e-paper recovery or prior card must remain visible without panel power.
- Rail retention must match the already-qualified normal sleep path.
- Recovery must remain explicit and adult-controlled.
- Sleep behavior must follow canonical ESP-IDF and the pinned ESP-HAL.

## Considered Options

- Keep terminal failures awake.
- Enter timed or RTC-alarm sleep and retry.
- Drive the board power latch low.
- Enter deep sleep without a wake source.

## Decision Outcome

Chosen option: "Enter deep sleep without a wake source."

After one bounded display attempt, non-provisioning terminal failures:

1. log one stable code, diagnostic bit, attempt count, and rail verdict;
2. switch the panel rail off;
3. retain GPIO6 and GPIO17 high and GPIO42 low;
4. enter `Rtc::sleep_deep(&[])`; and
5. remain asleep until external reset or power cycling.

Production rejects wake causes other than cold/reset and RTC `Ext0` before
rendering a daily card. A panel failure preserves the prior e-paper frame
instead of claiming that a failed panel path displayed its own recovery screen.

When the RTC bus is available but its value is invalid, setup mode remains
awake and exposes only bounded wired provisioning. If board initialization
fails before provisioning can operate, the `RTC` screen is attempted once and
the same no-wake terminal is used.

### Consequences

- Good, because terminal software failures no longer drain the battery while
  waiting for an adult.
- Good, because no timer or RTC source can turn a failure into a retry loop.
- Good, because the terminal reuses the qualified rail-retention boundary.
- Good, because an external reset is consistent with the displayed adult
  recovery actions.
- Bad, because wired diagnostics are unavailable after sleep; the one log line
  must be captured before USB disappears.
- Bad, because a device in terminal sleep cannot recover without physical
  adult intervention.
- Bad, because invalid RTC setup remains an intentional active-power exception
  while provisioning is available.

### Confirmation

CI must build release firmware and the RTC, panel, and alarm policy-injection
images with the locked ESP32-S3 toolchain. Host tests must continue to prove
unique bounded policies, recovery screens, corrupt-pack rejection, and the
unaffected 151-card path.

On the physical V2 board, each policy injection must log once, remove USB
within 30 seconds, remain absent for a bounded 60-second observation, and
retain the expected recovery or prior frame. Battery-side timing/current
evidence remains part of release qualification.

## Pros and Cons of the Options

### Keep failures awake

- Good, because logs remain continuously observable.
- Bad, because active current is unbounded.

### Sleep and retry

- Good, because transient failures might recover automatically.
- Bad, because repeated boots or refreshes violate the bounded-failure
  contract and can consume more energy than staying asleep.

### Turn the board power latch off

- Good, because it can minimize board power.
- Bad, because its recovery semantics differ from the qualified deep-sleep
  path and depend on board-level button behavior.

### Deep sleep without wake sources

- Good, because ESP-IDF defines it as indefinite sleep until external reset.
- Good, because it reuses the existing rail-retention implementation.
- Bad, because recovery always requires physical adult action.

## More Information

- [ADR 0004](0004-use-esp-idf-aligned-rtc-deep-sleep.md)
- [Bounded failure and recovery contract](../hardware/failure-recovery.md)
- [ESP-IDF entering Deep-sleep](https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32s3/api-reference/system/sleep_modes.html#entering-deep-sleep)
- [Pinned ESP-HAL deep-sleep implementation](https://github.com/esp-rs/esp-hal/blob/434755e0447fc1a4ba30fd84da3cf746ec082e00/esp-hal/src/rtc_cntl/mod.rs#L389-L402)

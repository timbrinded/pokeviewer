---
status: accepted
date: 2026-07-28
decision-makers:
  - Project maintainer
---

# Use ESP-IDF-aligned RTC deep sleep

## Context and Problem Statement

ADR 0003 deliberately kept production awake until each low-power boundary
could be qualified independently. The connected board subsequently proved
timer-only deep sleep, PCF85063 alarm assertion, GPIO5 active-low signalling,
and one alarm-driven EXT0 wake.

The failing implementation differed from canonical ESP-IDF in two places:
ESP-HAL 1.1.1 asserted `slp_wakeup` while requesting sleep, and the local
GPIO42 shim enabled global digital-pad autohold in addition to GPIO42's
per-pin hold bit.

Should release firmware continue polling awake, carry local sleep-register
workarounds, or use the reviewed ESP-HAL correction and an ESP-IDF-equivalent
per-pin hold?

## Decision Drivers

- Make canonical ESP-IDF the behavioural specification.
- Preserve the passive daily 07:00 product contract.
- Retain GPIO6 high, GPIO17 high, and GPIO42 low during deep sleep.
- Avoid broad global pad controls that can affect flash, USB, or power pins.
- Keep setup and terminal-failure modes observable and recoverable.
- Pin every unreleased dependency revision exactly.

## Considered Options

- Keep the staged awake polling runtime.
- Reimplement private ESP32-S3 sleep entry registers locally.
- Pin the reviewed ESP-HAL ESP-IDF-alignment commit and correct the local
  GPIO42 per-pin hold.

## Decision Outcome

Chosen option: "Pin the reviewed ESP-HAL ESP-IDF-alignment commit and correct
the local GPIO42 per-pin hold."

Release firmware now:

1. validates the RTC and renders one daily frame;
2. configures the fixed PCF85063 07:00 alarm;
3. re-reads the RTC and restarts once if refresh or alarm configuration crossed
   the planned boundary;
4. verifies GPIO5 is high before sleep;
5. retains GPIO6 high and GPIO17 high with their RTC per-pin holds;
6. retains GPIO42 low with only its documented digital per-pin hold bit;
7. enables the GPIO5 RTC pull-up and enters active-low EXT0 deep sleep; and
8. boots at the alarm, renders the new day, and plans the next strictly future
   07:00 alarm.

The project pins ESP-HAL commit
`434755e0447fc1a4ba30fd84da3cf746ec082e00`, the merge commit for upstream
PR 5807, until a crates.io release containing the same correction is qualified.
The matching ESP-RS packages come from that same commit to keep the native
`esp_rom_sys` link source unique.

Setup mode remains awake to serve bounded wired RTC provisioning. Classified
terminal failures remain awake rather than entering an unobservable failure
loop.

### Consequences

- Good, because sleep entry follows the reviewed ESP-IDF-aligned implementation.
- Good, because GPIO42 retention affects only GPIO42.
- Good, because timer, alarm assertion, and EXT0 wake each have a bounded
  conformance diagnostic.
- Bad, because release builds temporarily depend on exact Git revisions rather
  than crates.io-only packages.
- Bad, because battery runtime remains unqualified until battery-side current
  measurements are recorded.
- Bad, because a cold or manual reset still refreshes the retained e-paper
  frame; retained-card optimization is deferred.

### Confirmation

The decision was accepted after these connected-board observations:

- timer-only sleep: one ten-second sleep interval, timer wake, no reset cycle;
- alarm assertion while awake: AF asserted at `07:00:00`, GPIO5 went low, and
  clearing AF released GPIO5 high; and
- integrated wake: one sleep interval until the synthetic 07:00 boundary,
  retained result `wake_cause=Ext0`, and `alarm_was_pending=true`.
- release path: after restoring and reading back the real local RTC, production
  firmware refreshed once, disappeared from USB on deep-sleep entry, and did
  not re-enumerate during the bounded 45-second observation.

CI must build the release firmware and all three diagnostic binaries with the
locked ESP32-S3 toolchain. Battery-side current, passive-image photographs, and
the release battery-capacity calculation remain separate release gates.

## Pros and Cons of the Options

### Keep awake polling

- Good, because it is already stable and observable.
- Bad, because it cannot satisfy the battery-operated product contract.

### Reimplement sleep entry locally

- Good, because it could keep all dependencies on crates.io.
- Bad, because it duplicates private SoC sequencing already reviewed upstream.

### Pin the upstream correction

- Good, because it preserves provenance and matches canonical ESP-IDF.
- Good, because the full revision is immutable and reproducible.
- Bad, because the monorepo pin also selects matching ESP-RS transitive
  packages.

## More Information

- [ADR 0003](0003-stage-an-awake-runtime-before-deep-sleep-integration.md)
- [RTC wake and deep-sleep qualification](../hardware/deep-sleep-qualification.md)
- [07:00 runtime state machine](../hardware/wake-sleep-state-machine.md)
- [ADR 0005](0005-use-no-wake-deep-sleep-for-terminal-failures.md)
  supersedes the awake terminal-failure behavior; normal RTC wake is unchanged.
- [ESP-HAL PR 5807](https://github.com/esp-rs/esp-hal/pull/5807)
- [ESP-IDF sleep implementation](https://github.com/espressif/esp-idf/blob/v5.5.1/components/esp_hw_support/sleep_modes.c)

---
status: superseded by ADR-0004
date: 2026-07-28
decision-makers:
  - Project maintainer
---

# Stage an awake runtime before deep-sleep integration

Superseded by
[ADR 0004](0004-use-esp-idf-aligned-rtc-deep-sleep.md) after the isolated
timer, alarm-assertion, and GPIO5 EXT0 gates passed on 2026-07-28.

## Context and Problem Statement

The integrated release path rendered the correct card and removed power from
the panel, but its attempted deep-sleep transition was followed by USB
re-enumeration and another boot about 2.3 seconds later. USB disappearance
proved neither a stable sleep nor the source of the next reset. Repeatedly
changing alarm, wake, pad-hold, and sleep behavior together made the physical
result harder to isolate.

Should deep sleep remain on the production bring-up path, should the project
change frameworks, or should the runtime stay awake while the already working
subsystems and the low-power path are qualified independently?

## Decision Drivers

- Establish a stable, observable baseline before optimizing battery use.
- Preserve the accepted passive 07:00 product behavior as the final target.
- Change one hardware boundary at a time and require a live A/B observation.
- Keep failures from becoming reset, refresh, or sleep loops.
- Reuse the pinned Rust, ESP-HAL, ESP-IDF, and Waveshare evidence rather than
  changing frameworks without evidence.

## Considered Options

- Continue debugging the fully integrated alarm, wake, and deep-sleep path.
- Replace the Rust firmware stack before completing hardware isolation.
- Stage an awake production runtime and qualify sleep in a dedicated diagnostic.

## Decision Outcome

Chosen option: "Stage an awake production runtime and qualify sleep in a
dedicated diagnostic", because it gives the project a simple known-good
baseline without changing the final passive product contract.

The staged runtime:

1. renders exactly one frame per boot;
2. puts the panel controller to sleep and switches the panel rail off;
3. keeps GPIO17 high and GPIO42 low as ordinary outputs;
4. keeps the ES8311 software-suspended;
5. polls the RTC every 30 seconds without configuring or inspecting its alarm;
6. performs one software restart when the strictly future 07:00 boundary is
   reached; and
7. remains awake on RTC polling or terminal application failures.

Wake-cause interpretation, retained-card inference, RTC alarm configuration,
GPIO wake sources, pad holds, and MCU deep sleep are not part of this staged
production path. The `pokeviewer-sleep-diagnostic` remains the isolated place
to qualify those effects.

This ADR complements rather than supersedes
[ADR 0002](0002-use-a-passive-0700-display-day.md). ADR 0002 remains the final
product outcome; this record changes only the order in which it is brought up.

### Consequences

- Good, because display, RTC reads, rail levels, and scheduling can be observed
  without a sleep/reset ambiguity.
- Good, because an RTC polling failure retains the useful e-paper card and
  retries without resetting.
- Good, because deep-sleep experiments cannot destabilize the ordinary runtime.
- Bad, because the staged runtime is unsuitable for battery-life qualification
  and cannot be released as v1.
- Bad, because daily rollover temporarily uses a software reset and therefore
  refreshes after every boot.

### Confirmation

The awake baseline is accepted only after both physical checks pass:

- one boot, one full refresh, continuous USB presence, and no reset or
  re-enumeration for ten minutes; and
- a synthetic time just before 07:00 produces exactly one restart and refresh,
  after which USB remains continuously present with no further reset for ten
  minutes.

If the awake baseline still resets, stop sleep work and reduce the hardware path
to the last proven layer, starting with display-only operation. Do not explain
an observation by USB disappearance alone.

The staged runtime may exit only after separate live tests prove, in order:
display refresh, battery latch retention, panel controller hibernate and rail
off, timer-only MCU sleep, RTC alarm assertion and wake, then the integrated
daily path. Each stage must preserve its sanitized logs and rejected
hypotheses.

On 2026-07-28 both awake gates passed on the connected V2 board:

- the ordinary boot remained continuously present over USB for 607 seconds
  with zero state changes; and
- the synthetic near-07:00 run made one software reset and refresh, planned
  the following day's strictly future boundary, then remained continuously
  present for 608 seconds with zero USB state changes.

These observations qualify the awake baseline only. They do not qualify deep
sleep, RTC alarm wake, battery operation, or v1 release.

## Pros and Cons of the Options

### Continue debugging the fully integrated path

- Good, because it aims directly at the final low-power behavior.
- Bad, because simultaneous alarm, mux, hold, wake, and sleep effects obscure
  the cause of a reset.

### Replace the Rust firmware stack

- Good, because another framework might offer different examples and tooling.
- Bad, because the current Rust stack already renders, reads the RTC, controls
  the rails, and builds for the target; no evidence identifies the language or
  framework as the failure.

### Stage an awake runtime and isolate sleep

- Good, because it follows an observable bring-up sequence with one new effect
  per stage.
- Good, because ESP-IDF and vendor patterns can guide the isolated boundary
  without requiring a framework rewrite.
- Bad, because it intentionally delays power optimization.

## More Information

- [On-device daily-card integration](../development/on-device-integration.md)
- [RTC wake and deep-sleep qualification](../hardware/deep-sleep-qualification.md)
- [07:00 runtime state machine](../hardware/wake-sleep-state-machine.md)
- [ADR 0004](0004-use-esp-idf-aligned-rtc-deep-sleep.md)

---
status: accepted
date: 2026-10-01
decision-makers:
  - Project maintainer
---

# Re-check the battery every three hours and after a BOOT restart

## Context and Problem Statement

[ADR 0009](0009-use-a-wake-gated-low-voltage-battery-state.md) commits the
battery state only at the 07:00 alarm wake. In use, this has two problems:

- after a charge, `CHARGE!` stays on the card until the next 07:00 wake, even
  when the battery is full; and
- a battery that crosses the threshold shortly after 07:00 is not reported
  for almost a day.

The board still cannot sense USB power, and the charger `STAT` output drives
only the orange LED, so firmware cannot tell a charging reading from a
battery reading.

How often should firmware re-check the battery, given that every check costs
energy?

## Decision Drivers

- Clear `CHARGE!` soon after charging.
- Keep the card limited to the three battery states; do not add a level.
- Keep the extra energy small compared with the sleep floor.
- Never refresh the panel unless the displayed state changes.

## Considered Options

- Keep the once-a-day 07:00 commit.
- Re-check every hour.
- Re-check every three hours, and on each `BOOT` restart.

## Decision Outcome

Chosen option: "Re-check every three hours, and on each `BOOT` restart".

Daily sleep adds a three-hour ESP32-S3 RTC timer to its EXT1 sources. A timer
wake samples and commits the battery under the ADR 0009 thresholds and
hysteresis. It refreshes the panel only if the committed state differs from the
displayed one; otherwise it sleeps again without touching the panel. A
one-second `BOOT` restart also samples and commits before it redraws.

The 07:00 alarm keeps its commit. `PWR` sessions, invalid-RTC setup, and
terminal sleep still do not sample. The USB `get-battery` command still reports
the retained snapshot.

Estimated cost, before measurement: each check is a full boot of about
0.3–0.5 s at about 40–50 mA, or 0.004–0.008 mAh. Eight checks add
0.03–0.06 mAh per day, about 2–6 % of an estimated 1–2 mAh/day sleep floor.
Hourly checks would cost three times as much to clear the warning two hours
sooner.

### Consequences

- Good, because `CHARGE!` clears within three hours of charging, or at once
  after a `BOOT` restart.
- Good, because the panel refreshes only when the warning changes.
- Bad, because a check while USB is connected reads the charger's voltage and
  can clear `CHARGE!` before the battery is full. The orange charger LED, not
  the card, shows when charging is complete.
- Bad, because eight extra boots a day cost a few percent of battery life.

### Confirmation

Host tests cover timer-wake classification, `BOOT` restart sampling, and
redrawing only on a state change. A device check confirms one timer wake that
sleeps without a refresh and one `BOOT` restart that clears `CHARGE!`.

## More Information

- Supersedes the alarm-only commit gate of
  [ADR 0009](0009-use-a-wake-gated-low-voltage-battery-state.md). Its states,
  thresholds, hysteresis, and invalid-sample rules remain.
- [Firmware runtime](../hardware/wake-sleep-state-machine.md#battery-state)

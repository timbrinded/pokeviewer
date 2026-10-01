---
status: accepted
date: 2026-08-10
decision-makers:
  - Project maintainer
---

# Use a wake-gated low-voltage battery state

## Context and Problem Statement

Pokeviewer v1.1.0 converted cell voltage to a coarse percentage with a generic
LiPo open-circuit-voltage table. The selected cell has no validated
voltage-to-capacity curve, and load, temperature, age, charger state, and USB
power can change the observed voltage. The percentage therefore implied more
capacity information than the device can support.

The display still needs a clear recharge warning. A PWR parent session runs
under different power conditions from the scheduled battery path. It must not
sample or replace the state that the retained daily card represents.

Should firmware keep the generic percentage, show an immediate voltage-based
state after every wake, or use a wake-gated low-voltage state?

## Decision Drivers

- Do not claim precise battery capacity without cell characterization.
- Keep one clear recharge warning with hysteresis.
- Reject implausible ADC results without clearing a prior recharge condition.
- Keep the retained card and USB report tied to the scheduled observation.
- Keep transition logic deterministic and host-testable.
- Permit one bounded DMM comparison without making it a capacity calibration.

## Considered Options

- Keep the generic LiPo percentage estimate.
- Commit a voltage-based state after every wake.
- Use a wake-gated low-voltage state.

## Decision Outcome

Chosen option: "Use a wake-gated low-voltage state."

The product has exactly three battery states: `Normal`, `Recharge`, and
`Unavailable`. A plausible cell voltage below 3,750 mV enters `Recharge`. A
later plausible voltage at or above 3,850 mV clears it to `Normal`. Plausible
values are 2,500 mV through 4,500 mV inclusive.

`Normal` renders no battery text or icon. `Recharge` renders the existing
lightning icon and `CHARGE!`. `Unavailable` renders `BAT ?`.

If an invalid scheduled observation follows a retained `Recharge` state,
firmware preserves the complete prior snapshot. Otherwise, it commits
`Unavailable` with `0` mV. The firmware does not convert millivolts to a
percentage or capacity.

Only an observation taken after a validated RTC alarm wake commits the retained
scheduled snapshot. Reset, invalid-RTC, and PWR parent-session paths do not
sample or replace it. The display renders the committed snapshot. USB reports
the same retained snapshot; it does not take or expose a new PWR-session sample.

### Consequences

- Good, because the UI makes no unsupported state-of-charge claim.
- Good, because separate enter and clear thresholds prevent state chatter.
- Good, because a PWR session cannot replace the scheduled battery result.
- Good, because the USB value can support bounded voltage diagnostics.
- Bad, because `Normal` does not distinguish a partly charged cell from a full
  cell.
- Bad, because a stale retained snapshot can differ from the present voltage.
- Bad, because thresholds still depend on board ADC and power-path behavior.
- Bad, because a final DMM comparison and RTC-versus-PWR device check remain.

### Confirmation

Host tests must cover every threshold transition, RTC-alarm commit gate,
reset/PWR non-commit path, and both invalid scheduled-observation branches.
Reviewed framebuffer goldens must cover `Normal`, `Recharge`, and
`Unavailable`.

Final device confirmation requires the retained USB value to be within 150 mV
of one DMM reading at the cell terminals. It also requires one bounded
comparison between a validated RTC alarm wake and a PWR parent session. These
checks confirm voltage reporting and commit gating; they do not calibrate
capacity.

## Pros and Cons of the Options

### Keep the generic LiPo percentage estimate

- Good, because it gives users more apparent detail.
- Bad, because the selected cell has no validated voltage-to-capacity curve.
- Bad, because the apparent precision is not a capacity measurement.

### Commit a voltage-based state after every wake

- Good, because USB can report the most recent observation.
- Bad, because PWR and USB power can replace a battery-mode scheduled result.
- Bad, because the retained card can then describe a different observation.

### Use a wake-gated low-voltage state

- Good, because it preserves a clear warning without a capacity claim.
- Good, because the retained display and USB response use one scheduled
  snapshot.
- Bad, because the snapshot can remain stale until the next validated RTC alarm
  wake.

## More Information

- Supersedes [ADR 0007: Use a generic LiPo OCV battery estimate](0007-use-a-generic-lipo-ocv-battery-estimate.md)
- [Product contract](../product-contract.md)
- [Battery state and runtime scope](../hardware/battery-sizing.md)
- [Wake, parent-session, and 07:00 state machine](../hardware/wake-sleep-state-machine.md)
- [USB provisioning protocol v1](../usb-protocol-v1.md)

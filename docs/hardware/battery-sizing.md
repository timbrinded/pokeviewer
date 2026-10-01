# Battery state and runtime scope

Pokeviewer v1.2.0 shows one battery state: `Normal`, `Recharge`, or
`Unavailable`. It does not show a percentage or make a battery runtime or
precise capacity claim. Pokeviewer v1.1.0 used a coarse generic LiPo OCV
percentage estimate; [ADR 0007](../decisions/0007-use-a-generic-lipo-ocv-battery-estimate.md)
keeps that historical decision.

The supported V2 board connects the cell monitor to GPIO4 through a 2:1
divider. Firmware uses ADC1 at 11 dB attenuation with ESP-HAL
`AdcCalCurve`. After it validates an RTC alarm wake, it:

1. waits 50 ms;
2. discards the first conversion;
3. takes 16 calibrated millivolt samples at 2 ms intervals;
4. sorts the samples and averages the two middle values;
5. applies the 2:1 divider; and
6. rejects values outside 2,500 mV through 4,500 mV inclusive.

A plausible sample below 3,750 mV enters `Recharge`. A later plausible sample
at or above 3,850 mV clears it to `Normal`. The 100 mV hysteresis prevents
state changes near one threshold. If an invalid scheduled observation follows
a retained `Recharge` state, firmware preserves the complete prior snapshot.
Otherwise, it commits `Unavailable` with `0` mV.

Only the battery observation from a validated RTC alarm wake commits the
retained scheduled snapshot used by the card and USB report. Reset, invalid
RTC, and PWR parent-session paths do not sample or replace it. `pokeviewerctl
get-battery` reads that retained state and millivolt value; it does not take a
new sample.

Load, temperature, cell age, charger state, protection cutoff, ADC tolerance,
and USB power can change the measured voltage. Firmware does not use the state
or millivolt value to stop charging, disconnect the cell, or enforce a safety
limit.

Manual discharge testing, runtime certification, charger certification, and a
universal capacity claim are out of scope. Final hardware confirmation requires
the retained USB value to be within 150 mV of one DMM reading at the cell
terminals. It also requires one bounded comparison of an RTC alarm wake with a
PWR parent session.

# 07:00 runtime state machine

- Status: awake-first runtime and physical stability gates passed; deep sleep pending
- Delivery issue: [P19 / #20][issue-20]
- Last reviewed: 2026-07-28

During staged bring-up, normal firmware uses this deliberately simple loop:

```text
boot/reset
  -> hold the shared-bus audio rail on and software-suspend the ES8311
  -> validate RTC
  -> derive current display day and next strict 07:00
  -> refresh exactly once
  -> panel sleep and panel rail off; audio rail remains on with codec suspended
  -> remain awake and poll RTC every 30 seconds
  -> at the planned 07:00 boundary, software reset exactly once
```

Normal awake mode does not configure or inspect the PCF85063 alarm, inspect the
ESP wake cause, infer a retained card, configure GPIO wake, hold pads, or enter
MCU sleep. RTC polling failures retain the current card, log once per
consecutive failure period, and retry without resetting.

Before 07:00 the selection remains the prior calendar date, including its
weekday. At exactly 07:00 the current date is selected. `next_rollover`
calculates the first 07:00 strictly after the current reading, including
month, year, leap-day, and 151-day schedule boundaries. After the rollover
reset, the freshly read time plans tomorrow's strictly future boundary,
preventing a same-boundary reset loop.

Setup mode is intentionally different: an invalid RTC always refreshes the
adult setup screen and keeps only bounded wired provisioning active. A valid
set/read-back restarts into the normal state machine.

Host tests cover:

- `06:59:59` retaining the prior display date and targeting the same-day 07:00;
- the exact 07:00 observation requesting one restart;
- post-restart planning of tomorrow's boundary;
- RTC poll failures retaining the card without requesting a restart;
- cold-reset convergence;
- month, year, leap-day, maximum-range, and schedule-cycle boundaries; and
- invalid RTC routing through the setup state.

The connected V2 board has proven RTC reads, daily rendering, panel-controller
sleep, and panel rail-off. A prior 15-second trace showed USB re-enumeration and
another boot about 2.3 seconds after the attempted sleep transition, so USB
disappearance is not accepted as a deep-sleep pass.

On 2026-07-28, the ordinary awake boot remained continuously present over USB
for 607 seconds with zero state changes. A synthetic near-07:00 run then made
one software reset, rendered the new card once, planned the following day's
boundary, and remained continuously present for 608 seconds with zero state
changes.

The separate sleep diagnostic must still prove timer sleep, RTC alarm
assertion, GPIO5 wake, and the integrated low-power loop before v1 release. See
[ADR 0003](../decisions/0003-stage-an-awake-runtime-before-deep-sleep-integration.md).

[issue-20]: https://github.com/timbrinded/pokeviewer/issues/20

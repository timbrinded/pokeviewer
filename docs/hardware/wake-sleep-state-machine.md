# 07:00 runtime state machine

- Status: RTC alarm deep sleep integrated and physically qualified
- Delivery issue: [P19 / #20][issue-20]
- Last reviewed: 2026-07-28

Normal firmware uses this passive daily loop:

```text
boot/reset
  -> hold the shared-bus audio rail on and software-suspend the ES8311
  -> validate RTC
  -> derive current display day and next strict 07:00
  -> refresh exactly once
  -> panel sleep and panel rail off; audio rail remains on with codec suspended
  -> configure the fixed PCF85063 07:00 alarm
  -> re-read RTC; restart once if refresh/configuration crossed the boundary
  -> retain GPIO6 high, GPIO17 high, and GPIO42 low
  -> deep sleep with GPIO5 active-low EXT0 wake
  -> at the alarm, boot and repeat for the new display day
```

The PCF alarm flag is cleared and configured before every sleep. GPIO5 must be
high before sleep; firmware refuses to enter an immediate-wake loop while the
interrupt remains asserted. GPIO5 uses its RTC-domain pull-up during sleep.
GPIO6 and GPIO17 use RTC per-pin holds. GPIO42 uses only its digital per-pin
hold bit, matching ESP-IDF `gpio_hold_en`; global digital-pad autohold is
forbidden.

Before 07:00 the selection remains the prior calendar date, including its
weekday. At exactly 07:00 the current date is selected. `next_rollover`
calculates the first 07:00 strictly after the current reading, including
month, year, leap-day, and 151-day schedule boundaries. After the rollover
boot, the freshly read time plans tomorrow's strictly future boundary,
preventing a same-boundary reset loop.

Setup mode is intentionally different: an invalid RTC always refreshes the
adult setup screen and keeps only bounded wired provisioning active. A valid
set/read-back restarts into the normal state machine.

Host tests cover:

- `06:59:59` retaining the prior display date and targeting the same-day 07:00;
- refresh or alarm configuration crossing 07:00 requesting one restart;
- post-wake planning of tomorrow's boundary;
- cold-reset convergence;
- month, year, leap-day, maximum-range, and schedule-cycle boundaries; and
- invalid RTC routing through the setup state.

The connected V2 board has proven RTC reads, daily rendering, panel-controller
sleep, panel rail-off, timer deep sleep, PCF alarm assertion, GPIO5 release
after clearing AF, and one alarm-driven EXT0 wake.

On 2026-07-28, the ordinary awake boot remained continuously present over USB
for 607 seconds with zero state changes. A synthetic near-07:00 run then made
one software reset, rendered the new card once, planned the following day's
boundary, and remained continuously present for 608 seconds with zero state
changes.

Battery-side current and retained-image evidence remain release gates. See
[ADR 0004](../decisions/0004-use-esp-idf-aligned-rtc-deep-sleep.md).

[issue-20]: https://github.com/timbrinded/pokeviewer/issues/20

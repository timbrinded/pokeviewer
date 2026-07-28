# 07:00 wake-refresh-sleep state machine

- Status: implemented; deep-sleep entry passed; scheduled wake and battery evidence pending
- Delivery issue: [P19 / #20][issue-20]
- Last reviewed: 2026-07-27

The release firmware has one scheduled active period per local display day:

```text
boot/wake
  -> hold the shared-bus audio rail on and software-suspend the ES8311
  -> validate RTC
  -> derive current display day and next strict 07:00
  -> refresh only for reset/unknown state or an asserted RTC alarm
  -> clear and arm the PCF85063 daily 07:00 alarm
  -> panel sleep and panel rail off; audio rail remains on with codec suspended
  -> enable GPIO5 RTC-domain pull-up and disable its RTC-domain pull-down
  -> ESP32-S3 deep sleep on active-low RTC_INT
```

Before 07:00 the selection remains the prior calendar date, including its
weekday. At exactly 07:00 the current date is selected. `next_rollover`
calculates the first 07:00 strictly after the current reading, including
month, year, leap-day, and 151-day schedule boundaries.

The PCF85063 alarm flag is the hardware freshness signal. A valid active-low
RTC wake with the flag asserted refreshes once. An EXT0 wake without the flag
is treated as a duplicate/spurious wake: the retained frame is left untouched,
the next daily alarm is re-armed, and the device sleeps again. A cold boot,
software reset, or power restoration has no trustworthy retained-card record,
so it converges by rendering the correct current display day once. This avoids
unsafe persistent mutable state and still makes every supported wake decision
deterministic.

Setup mode is intentionally different: an invalid RTC always refreshes the
adult setup screen and keeps only bounded wired provisioning active. A valid
set/read-back restarts into the normal state machine.

Host tests cover:

- `06:59:59` retaining the prior display date and targeting the same-day 07:00;
- the exact 07:00 transition changing the selected card once;
- duplicate-wake retention when the known display date already matches;
- cold-reset convergence;
- month, year, leap-day, maximum-range, and schedule-cycle boundaries; and
- invalid RTC routing through the setup state.

After the GPIO5 RTC-mux cleanup, the connected V2 board rendered framebuffer
CRC `d227338a` and entered deep sleep; the USB connection disappeared as
expected. That entry does not qualify the subsequently explicit RTC-domain
GPIO5 pull-up. The scheduled RTC wake/reboot is still pending. Timestamped
alarm transitions, retained-card photographs, refresh duration, battery
polarity, awake-idle current, and deep-sleep current also remain required.

[issue-20]: https://github.com/timbrinded/pokeviewer/issues/20

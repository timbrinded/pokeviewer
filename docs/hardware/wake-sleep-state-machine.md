# Wake, parent-session, and 07:00 state machine

- Status: v1.2.0 implementation complete; battery device confirmation pending
- Last reviewed: 2026-08-10

Release firmware uses one ESP32-S3 EXT1 `ANY_LOW` wake source:

- GPIO5: active-low PCF85063 daily alarm; and
- GPIO18: active-low PWR button.

Firmware reads `EXT_WAKEUP1_STATUS` before it configures the next sleep. It
combines the GPIO5 status bit with the PCF85063 alarm flag. It does not infer
wake intent from reset history or USB enumeration.

## Normal daily path

```text
reset or EXT1 wake
  -> restore GPIO5 and GPIO18 from RTC control
  -> hold the shared-bus audio rail low and suspend the ES8311
  -> validate the RTC and wake evidence
  -> derive the current display day and the next strict 07:00
  -> sample the GPIO4 battery divider only after a validated RTC alarm wake
  -> commit the resulting battery snapshot
  -> refresh exactly once when reset or a valid RTC alarm requires it
  -> panel sleep and panel rail off
  -> configure the fixed PCF85063 07:00 alarm
  -> re-read the RTC
  -> restart once if refresh or alarm setup crossed the boundary
  -> retain GPIO6 high, GPIO17 high, and GPIO42 low
  -> configure GPIO5 and GPIO18 with RTC pull-ups
  -> enter active-low EXT1 deep sleep
```

The PCF alarm flag is cleared and configured before sleep. GPIO5 and GPIO18
must both be high. Firmware refuses sleep if a configured wake input remains
low. GPIO6 and GPIO17 use RTC per-pin holds. GPIO42 uses only its documented
digital per-pin hold bit, which matches ESP-IDF `gpio_hold_en`.

Before 07:00, the selection remains the prior calendar date, including its
weekday. At exactly 07:00, the current date is selected. `next_rollover`
calculates the first 07:00 strictly after the current reading. This includes
month, year, leap-day, and 151-day schedule boundaries.

The retained scheduled battery snapshot has one of three states: `Normal`,
`Recharge`, or `Unavailable`. A plausible value below 3,750 mV enters
`Recharge`; a later plausible value at or above 3,850 mV clears it. If a
scheduled observation is invalid while the retained state is `Recharge`, the
complete prior snapshot remains. Otherwise, the scheduled observation commits
`Unavailable` with `0` mV. Reset and invalid-RTC paths do not sample or commit.

## PWR path

A PWR tap wakes the ESP but does not refresh the panel. Firmware waits for the
button release and returns to the normal EXT1 sleep.

A continuous three-second PWR hold opens a 15-second USB frame gate. A valid
protocol frame starts a two-minute parent session. USB power without a valid
frame does not start the session.

The parent session first shows `SET TIME`. It can:

- read or set the RTC;
- read the retained scheduled battery snapshot;
- read diagnostics; or
- accept the confirmed storage command.

A successful RTC write is read back before firmware restarts, restores the
daily card, and returns to normal sleep. A session timeout also restores the
daily card when the RTC remains valid.

The PWR path does not take or commit a battery observation. The USB battery
command returns the retained scheduled state and millivolts, so USB power
cannot replace the value used by the retained card.

If GPIO5 and GPIO18 assert together, the daily refresh completes first.
Firmware then evaluates the continued PWR hold.

## Invalid RTC path

An invalid RTC always refreshes `SET TIME` and serves USB for two minutes. A
successful RTC write restarts normal operation. A timeout enters deep sleep
with GPIO18 as the only wake source. An invalid clock cannot cause an RTC alarm
wake.

## Storage path

Storage mode is available only in a PWR-gated parent session. Firmware:

1. shows `SET TIME`;
2. sends the successful protocol response;
3. writes the PCF85063 software-reset value and verifies oscillator-stop;
4. waits 100 ms for the response to leave USB;
5. drives and retains GPIO17 low; and
6. enters deep sleep with no ESP wake source.

USB can keep the ESP powered after GPIO17 drops. Disconnecting USB completes
the board power-off. The next PWR start shows `SET TIME`.

## Failure path

Pack, panel, alarm, and unexpected-wake failures make one bounded display
attempt and then enter no-wake deep sleep. They do not retry automatically.
The retained e-paper card or recovery screen remains visible.

## Test boundary

Host tests cover schedule boundaries, wake-source classification, RTC
read-back, storage authorization, battery filtering, hysteresis, commit
gating, invalid-sample retention, all 151 cards, and reviewed three-state
framebuffer goldens.

The V2 device qualification must prove:

- one GPIO5 alarm wake and status bit;
- one GPIO18 PWR wake and status bit;
- no visible change after a PWR tap;
- the three-second hold, frame gate, and two-minute session;
- daily-refresh-first ordering for simultaneous GPIO5 and GPIO18;
- storage-mode RTC invalidation, GPIO17 drop, and no-wake state; and
- a later PWR start into invalid-RTC setup.

Battery confirmation also requires one DMM comparison of the retained USB
millivolts and one bounded check that contrasts an RTC alarm commit with a PWR
parent session.

Manual current measurement and discharge testing remain outside scope. See
[ADR 0004](../decisions/0004-use-esp-idf-aligned-rtc-deep-sleep.md),
[ADR 0005](../decisions/0005-use-no-wake-deep-sleep-for-terminal-failures.md),
and
[ADR 0006](../decisions/0006-use-pwr-gated-parent-setup-and-storage-mode.md).
This battery contract is recorded in
[ADR 0009](../decisions/0009-use-a-wake-gated-low-voltage-battery-state.md).

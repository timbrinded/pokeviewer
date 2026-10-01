# Firmware runtime: wake, sleep, battery, and failures

This is the behavior of the release firmware (`pokeviewer-firmware`). The
decisions behind it are ADRs
[0002](../decisions/0002-use-a-passive-0700-display-day.md),
[0004](../decisions/0004-use-esp-idf-aligned-rtc-deep-sleep.md),
[0005](../decisions/0005-use-no-wake-deep-sleep-for-terminal-failures.md),
[0006](../decisions/0006-use-pwr-gated-parent-setup-and-storage-mode.md),
[0009](../decisions/0009-use-a-wake-gated-low-voltage-battery-state.md), and
[0010](../decisions/0010-use-boot-as-an-adult-restart-and-gpio3-as-a-status-light.md).

## Wake sources

Deep sleep uses one ESP32-S3 EXT1 `ANY_LOW` wake source:

- GPIO0: `BOOT` button, active low;
- GPIO5: PCF85063 alarm interrupt, active low; and
- GPIO18: `PWR` button, active low.

Firmware reads `EXT_WAKEUP1_STATUS` before it configures the next sleep. An
RTC wake needs both the GPIO5 status bit and the PCF85063 alarm flag. Firmware
does not infer wake intent from reset history or USB enumeration. It rejects
any wake other than cold boot, reset, or EXT1 on these three pins before it
can render a card.

## Daily path

```text
reset or EXT1 wake
  -> restore GPIO5 and GPIO18 from RTC control
  -> hold the shared-bus audio rail on (GPIO42 low) and suspend the ES8311
  -> send the SHTC3 sleep command
  -> validate the RTC and wake evidence
  -> derive the current display day and the next strict 07:00
  -> sample the battery only after a validated RTC alarm wake
  -> refresh once when reset or a valid RTC alarm requires it
  -> panel sleep and panel rail off
  -> configure the fixed PCF85063 07:00 alarm
  -> re-read the RTC; restart once if refresh or alarm setup crossed 07:00
  -> hold GPIO6 high, GPIO17 high, and GPIO42 low
  -> wait up to ten seconds for GPIO0, GPIO5, and GPIO18 to rise
  -> arm the lines that rose, with RTC-domain pull-ups
  -> enter EXT1 deep sleep
```

EXT1 `ANY_LOW` wakes at once from a line that is still low, so
`select_sleep_wake_sources` arms only lines that rose during the wait.
Firmware never waits indefinitely. A PCF85063 interrupt line that stays low
after the alarm flag clears is an `ALARM` failure, because the daily alarm
could never wake the device. GPIO6 and GPIO17 use RTC per-pin holds.
GPIO42 is not an RTC pin and uses only its digital per-pin hold bit, which
matches ESP-IDF `gpio_hold_en`. The [board contract](v2-board-contract.md)
explains why GPIO42 must stay low.

## Display day

Before 07:00, the display day is the previous calendar date, including its
weekday. At exactly 07:00, the current date is selected. `next_rollover`
returns the first 07:00 strictly after the current reading, across month,
year, leap-day, and 151-day schedule boundaries. The
[content-pack contract](../content-pack-v1.md#schedule-v1) defines the
schedule.

## Battery state

After a validated RTC alarm wake, and before the panel refresh, firmware reads
GPIO4 (a 2:1 divider) with ADC1 at 11 dB attenuation and ESP-HAL
`AdcCalCurve`. It waits 50 ms, discards one conversion, takes 16 calibrated
samples 2 ms apart, averages the middle two, and doubles the result. Values
from 2,500 mV through 4,500 mV are plausible.

| Observation | Retained state |
| --- | --- |
| plausible, below 3,750 mV | `Recharge` |
| plausible, at or above 3,850 mV | `Normal` |
| plausible, 3,750 mV to 3,849 mV | `Recharge` if the retained state is `Recharge`, otherwise `Normal` |
| implausible, retained state `Recharge` | the whole prior snapshot is kept |
| implausible, otherwise | `Unavailable` with `0` mV |

Only that scheduled observation commits the retained snapshot. Reset, `BOOT`,
invalid-RTC, and `PWR` paths do not sample or replace it. The card renders the
snapshot, and the USB `get-battery` command returns it without taking a new
sample. The value is not a fuel gauge and does not control charging, shutdown,
or safety. The board cannot sense USB power, so firmware cannot tell a
USB-powered reading from a battery reading.

## PWR path

A `PWR` tap wakes the ESP32-S3 but does not refresh the panel. Firmware waits
for release and sleeps again.

A continuous three-second hold turns the green LED on and opens a 15-second
USB frame gate. A valid protocol frame starts a two-minute parent session.
USB power without a valid frame does not start a session. The session first
shows `SET TIME`, then accepts read or set RTC, read battery, diagnostics, and
the confirmed storage command. A successful RTC write is read back, then
firmware restarts and redraws the card. A session timeout also redraws the
card when the RTC is valid.

If GPIO5 and GPIO18 assert together, the daily refresh runs first, then
firmware evaluates the continued `PWR` hold. The green LED turns off before
the device sleeps or restarts.

## BOOT restart path

A `BOOT` wake must stay held for one second, counted from wake. A shorter
press returns to the same sleep without a refresh; after a terminal failure,
that is the `BOOT`-only terminal sleep. A recognized hold flashes the green LED
once, waits for release, and runs the reset path: it reads the RTC and draws
the card, `SET TIME`, or the recovery screen. It does not sample the battery.
An alarm refresh or `PWR` session in the same wake takes precedence.

## Invalid RTC path

The RTC gate has two outcomes: `Ready` with a display day from one fresh,
validated reading, or `SetupRequired` with no date attached. Oscillator stop,
bus errors, impossible calendar fields, and years outside 2000–2099 all
require setup. Firmware never uses a last-known date or a default.

An invalid RTC draws the setup screen (`SET TIME`, `CONNECT USB`, `RTC RUN`,
`POKEVIEWERCTL`), turns the green LED on, and serves USB for two minutes. A
set command validates all fields before writing, and the read-back must pass
the same gate before firmware restarts into the daily path. A timeout sleeps
with GPIO0 and GPIO18 armed. An invalid clock cannot produce an alarm wake.

## Storage path

Storage mode is accepted only in a `PWR` parent session. Firmware:

1. shows `SET TIME`;
2. sends the successful protocol response;
3. software-resets the PCF85063 and verifies oscillator stop;
4. waits 100 ms for the response to leave USB;
5. drives and holds GPIO17 low; and
6. enters deep sleep with no wake source.

USB can keep the ESP32-S3 powered after GPIO17 drops. Disconnecting USB
completes the power-off. The next `PWR` press starts the board in setup.

## Failure path

Every expected failure has a code, a diagnostics bit, and at most one
automatic attempt per wake.

| Failure | Code | Flag | Attempts | Screen | Recovery |
| --- | --- | ---: | ---: | --- | --- |
| invalid, stopped, or unreadable RTC | `RTC` | `0x0001` | 0 | setup screen | set the RTC over USB |
| corrupt or incompatible pack | `PACK` | `0x0002` | 1 | `REFLASH` | reflash the release |
| panel init, refresh, or BUSY timeout | `PANEL` | `0x0004` | 1 | prior frame kept | check the panel, hold `BOOT` |
| daily alarm could not be armed | `ALARM` | `0x0008` | 1 | `RESET` | hold `BOOT` |
| unsupported wake source | `WAKE` | `0x0010` | 0 | `RESET` | hold `BOOT` |

Except for `RTC`, which keeps serving USB, a failure logs its code, flag,
attempt count, and rail state once, turns the panel rail off, holds the rails
as in normal sleep, and enters deep sleep with only GPIO0 armed. There is no
timer, alarm, or `PWR` wake and no automatic retry. A one-second `BOOT` hold
runs the failed path once more. If `BOOT` is still low after the ten-second
release wait, the terminal sleep arms no wake source.

The panel adapter polls BUSY every 10 ms for at most 500 polls. A failed
panel cannot draw its own error, so the prior frame stays visible. The
[recovery-screen evidence](../evidence/recovery-screens/README.md) holds the
exact images for the other codes.

## Verification

Host tests cover schedule boundaries, wake classification, RTC read-back,
storage authorization, battery filtering, hysteresis, commit gating,
invalid-sample retention, failure policies, all 151 cards, and the reviewed
framebuffer goldens.

The [board contract](v2-board-contract.md#physical-verification-status)
records what has been observed on the V2 board.

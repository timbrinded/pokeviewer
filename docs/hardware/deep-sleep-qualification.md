# RTC wake and deep-sleep qualification

This procedure proves the low-power hardware loop with a single diagnostic
frame. It does not estimate battery life from a board datasheet or promise a
runtime for an untested cell.

## Diagnostic behavior

The dedicated `pokeviewer-sleep-diagnostic` binary:

1. records the ESP32-S3 wake cause;
2. holds GPIO17 high and keeps the GPIO42 audio enable high;
3. validates the PCF85063 oscillator and clears any stale alarm flag;
4. avoids reconfiguring the alarm during the matching second after an RTC wake;
5. refreshes one identification frame and switches GPIO6 high to remove panel
   power;
6. verifies the active-low GPIO5 interrupt returned high;
7. holds GPIO6 and GPIO17 high through deep sleep; and
8. enters deep sleep with GPIO5 as an active-low RTC-domain wake source with a
   pull-up.

Avoiding alarm reconfiguration after an `Ext0` wake is deliberate. Clearing and
immediately re-enabling a still-matching 07:00:00 alarm could leave GPIO5 low
and create a reboot loop. The next day's match remains enabled in the RTC.

The binary rejects an `Ext0` wake without a matching PCF85063 alarm flag and
refuses to sleep while GPIO5 is still low. Wi-Fi, BLE, audio, SD, touch, and the
environment sensor are never initialized.

## Build and run

Use the pinned toolchain:

```sh
source .tools/export-esp.sh
CARGO_HOME="$PWD/.cargo-cache" \
  RUSTUP_HOME="$PWD/.rustup-cache" \
  cargo xtask sleep-diagnostic-build
CARGO_HOME="$PWD/.cargo-cache" \
  RUSTUP_HOME="$PWD/.rustup-cache" \
  cargo xtask sleep-diagnostic-flash
```

Provision a valid local datetime first. To avoid waiting until the next morning
during qualification, set the RTC to a documented test time immediately before
07:00, then restore the correct local time after the run.

Expected sanitized log sequence:

```text
sleep diagnostic ready; wake_cause=Undefined; RTC=YYYY-MM-DD 06:59:SS; alarm_was_pending=false; rails_off=true
sleep diagnostic ready; wake_cause=Ext0; RTC=YYYY-MM-DD 07:00:SS; alarm_was_pending=true; rails_off=true
```

There must be exactly one `Ext0` boot for that alarm. A repeated `Ext0` line
before the next calendar day is a failure.

## Image-retention evidence

Take a sanitized photo of the identification frame immediately before the
first sleep. After the RTC wake has returned to deep sleep, take a second photo
without touching or resetting the board. Record:

- readable text and border in both images;
- no unexpected blanking or partial refresh;
- GPIO6 high after each refresh; and
- elapsed time between the images.

## Current measurements

Measure at the battery input using a tool whose burden voltage and sample rate
are appropriate for both refresh peaks and low sleep current. Do not infer
battery-input current from USB current.

| Phase | Current | Duration | Conditions | Status |
| --- | ---: | ---: | --- | --- |
| refresh peak | — | — | panel powered, full refresh | pending |
| active average | — | — | boot through panel rail-off | pending |
| settled deep sleep | — | — | at least 60 s after rail-off | pending |
| one wake cycle | — | — | RTC alarm through next sleep | pending |

Let currents be in mA and durations in seconds. Calculate, rather than guess:

```text
active_mAh = active_mA * active_seconds / 3600
refresh_mAh = refresh_mA * refresh_seconds / 3600
sleep_seconds = 86400 - active_seconds - refresh_seconds
sleep_mAh = sleep_mA * sleep_seconds / 3600
daily_mAh = active_mAh + refresh_mAh + sleep_mAh
minimum_72h_mAh = 3 * daily_mAh / usable_capacity_fraction
```

`usable_capacity_fraction` must be justified for the exact protected cell,
temperature, age, cutoff voltage, and chosen reserve. Report the calculated
minimum alongside those assumptions; do not present it as universal runtime.

## Evidence status

| Acceptance check | Status | Evidence |
| --- | --- | --- |
| one alarm, one `Ext0` wake | pending | serial access required |
| passive image retention | pending | before/after photos required |
| refresh/active/sleep current | pending | battery-side measurement required |
| daily energy and 72-hour capacity | pending | depends on measured values |

The current `/dev/ttyACM0` permissions block flashing and monitoring. Restore
access through the host's normal serial-device group; do not make the node
world-writable. Follow the [privacy and evidence rules](../privacy-and-evidence.md)
before publishing logs or photos.

# RTC wake and deep-sleep qualification

This procedure proves the low-power hardware loop with a single diagnostic
frame. It does not estimate battery life from a board datasheet or promise a
runtime for an untested cell.

## Diagnostic behavior

The dedicated `pokeviewer-sleep-diagnostic` binary:

1. records the ESP32-S3 wake cause;
2. holds GPIO17 high, drives GPIO42 low, and applies the vendor ES8311
   software-suspend sequence;
3. validates the PCF85063 oscillator and clears any stale alarm flag;
4. avoids reconfiguring the alarm during the matching second after an RTC wake;
5. refreshes one identification frame and switches GPIO6 high to remove panel
   power;
6. verifies the active-low GPIO5 interrupt returned high;
7. holds GPIO6 and GPIO17 high and GPIO42 low through deep sleep; and
8. enables the GPIO5 RTC-domain pull-up, disables its RTC-domain pull-down, and
   enters deep sleep with GPIO5 as an active-low RTC-domain wake source;
9. after a valid RTC wake, revalidates the RTC and panel, prints one stable
   pass record, and remains awake for evidence capture and reflashing.

Avoiding alarm reconfiguration after an `Ext0` wake is deliberate: the
diagnostic clears AF, preserves the already configured daily comparison, and
stops after proving exactly one event. Clearing AF deasserts GPIO5, and the
alarm can assert again only when time next increments into a matching value.
The diagnostic remains awake after the validated wake so its pass record is
not lost when USB powers down again.

The binary rejects an `Ext0` wake without a matching PCF85063 alarm flag and
refuses to sleep while GPIO5 is still low. Audio capture and playback, Wi-Fi,
BLE, SD, touch, and the environment sensor are never initialized. The powered
ES8311 receives only its vendor software-suspend sequence.

GPIO5 is switched from the digital IO mux to the RTC_IO mux for `Ext0` wake.
The digital input pull-up used for the pre-sleep level check does not configure
the RTC_IO pull resistor. Firmware must explicitly enable the RTC-domain
pull-up and disable the RTC-domain pull-down before entering deep sleep. A
successful sleep entry alone does not prove that this wake input is biased
correctly; the one-alarm/one-wake observation is required.

GPIO42 is not an RTC-domain pin, so ESP-HAL cannot retain it with `RtcPin`.
The audited `pokeviewer-esp32s3-pad-hold` crate owns the single PAC operation
that changes GPIO42's documented digital hold bit under a critical section.
Firmware configures GPIO42 low before enabling per-pad auto-hold and restores
the low output before releasing the hold after wake. It never uses the
all-digital-pad force-hold control, which would also freeze flash and USB pins.
This follows Espressif's [GPIO hold contract][gpio-hold] and
[ESP32-S3 hold-mask mapping][hold-mask].

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
sleep diagnostic ready; wake_cause=Undefined; RTC=YYYY-MM-DD 06:59:SS; alarm_was_pending=false; panel_rail_off=true; audio_rail_on=true; audio_codec_suspended=true
sleep diagnostic passed; wake_cause=Ext0; RTC=YYYY-MM-DD 07:00:SS; alarm_was_pending=true; panel_rail_off=true; audio_rail_on=true; audio_codec_suspended=true
```

The RTC timestamp is intentionally present as test evidence. Use a documented
synthetic qualification date where possible and sanitize captured output
before publication.

There must be exactly one `Ext0` boot for that alarm. A repeated `Ext0` line is
a failure. Restore the correct local time and release firmware after capturing
the pass record.

## Image-retention evidence

Take a sanitized photo of the identification frame immediately before the
first sleep. After the RTC wake pass record, take a second photo without
touching or resetting the board. Record:

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
| settled deep sleep | — | — | at least 60 s after panel rail-off; audio rail powered, ES8311 software-suspended | pending |
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
| release-firmware deep-sleep entry | failed/unqualified | a 15-second trace re-enumerated and booted again after about 2.3 s |
| GPIO5 RTC-domain pull-up | pending | dedicated diagnostic sleep/wake run required |
| one alarm, one `Ext0` wake | pending | dedicated diagnostic run required |
| passive image retention | pending | before/after photos required |
| refresh/active/sleep current | pending | battery-side measurement required |
| daily energy and 72-hour capacity | pending | depends on measured values |

USB serial access, RTC reads, the release-firmware render, panel-controller
sleep, and panel rail-off have passed. USB disappearance by itself is not a
deep-sleep pass; the observed re-enumeration/reset must not be relabelled as a
wake. The dedicated timer-sleep and near-07:00 RTC-wake diagnostics,
battery-side measurements, battery polarity gate, and retained-image photos
remain pending. Follow the
[privacy and evidence rules](../privacy-and-evidence.md) before publishing logs
or photos.

[gpio-hold]: https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32s3/api-reference/peripherals/gpio.html
[hold-mask]: https://github.com/espressif/esp-idf/blob/v5.5.1/components/soc/esp32s3/gpio_periph.c

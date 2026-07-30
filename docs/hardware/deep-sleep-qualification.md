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
8. enables the RTC-domain pull-ups for GPIO5 and GPIO18, disables their
   RTC-domain pull-downs, and enters deep sleep with both pins in one
   active-low `Ext1` wake source;
9. after a valid RTC wake, revalidates the RTC and panel, records the verdict
   in RTC-retained memory, and remains awake for evidence capture and
   reflashing.

Avoiding alarm reconfiguration after an RTC `Ext1` wake is deliberate: the
diagnostic clears AF, preserves the already configured daily comparison, and
stops after proving exactly one event. Clearing AF deasserts GPIO5, and the
alarm can assert again only when time next increments into a matching value.
The diagnostic remains awake after the validated wake so its pass record is
not lost when USB powers down again.

The binary snapshots `EXT_WAKEUP1_STATUS` before it configures the next sleep.
It rejects an RTC `Ext1` wake without both the GPIO5 status bit and a matching
PCF85063 alarm flag. It also refuses to sleep while GPIO5 or GPIO18 is low.
Audio capture and playback, Wi-Fi, BLE, SD, touch, and the environment sensor
are never initialized. The powered ES8311 receives only its vendor
software-suspend sequence.

GPIO5 and GPIO18 are switched from the digital IO mux to the RTC_IO mux for
`Ext1` wake.
The digital input pull-up used for the pre-sleep level check does not configure
the RTC_IO pull resistor. Firmware must explicitly enable the RTC-domain
pull-up and disable the RTC-domain pull-down before entering deep sleep. A
successful sleep entry alone does not prove that this wake input is biased
correctly; the one-alarm/one-wake observation is required.

GPIO42 is not an RTC-domain pin, so ESP-HAL cannot retain it with `RtcPin`.
The audited `pokeviewer-esp32s3-pad-hold` crate owns the single PAC operation
that changes GPIO42's documented digital hold bit under a critical section.
Firmware configures GPIO42 low before setting that per-pin bit and restores the
low output before releasing it after wake. It does not enable global
`DG_PAD_AUTOHOLD_EN` or use all-digital-pad force hold. This is the direct
ESP-IDF `gpio_hold_en(GPIO42)` behavior and avoids affecting flash, USB, or
power-related pads. See Espressif's [GPIO hold contract][gpio-hold] and
[ESP32-S3 hold-mask mapping][hold-mask].

Sleep-entry behavior comes from the exact upstream ESP-HAL merge commit for
[PR 5807][esp-hal-5807], which aligns the request sequence with ESP-IDF:
request sleep without asserting `slp_wakeup`, then wait while the RTC clock
domain accepts deep sleep. Hardware experiments validate this translation;
they do not define the expected behavior.

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
sleep diagnostic ready; wake_cause=Undefined; RTC=YYYY-MM-DD 06:59:SS; alarm_was_pending=false; panel_rail_off=true; audio_power_low=true; audio_codec_suspended=true
sleep diagnostic passed; wake_cause=Ext1; rtc_status_bit=true; RTC=YYYY-MM-DD 07:00:SS; alarm_was_pending=true; panel_rail_off=true; audio_power_low=true; audio_codec_suspended=true
```

The RTC timestamp is intentionally present as test evidence. Use a documented
synthetic qualification date where possible and sanitize captured output
before publication.

There must be exactly one RTC `Ext1` boot for that alarm. A repeated `Ext1`
line is a failure. Restore the correct local time and release firmware after
you capture the pass record.

## Image-retention evidence

Take a sanitized photo of the identification frame immediately before the
first sleep. After the RTC wake pass record, take a second photo without
touching or resetting the board. Record:

- readable text and border in both images;
- no unexpected blanking or partial refresh;
- GPIO6 high after each refresh; and
- elapsed time between the images.

## Energy-measurement scope

Manual current measurement and discharge testing are not part of v1.1.0.
The release makes no battery-runtime claim. The on-screen battery value is a
generic, coarse estimate. It is not evidence of capacity, runtime, charging
performance, or a safe cutoff.

## Evidence status

| Acceptance check | Status | Evidence |
| --- | --- | --- |
| timer-only deep-sleep entry | passed | one ten-second sleep interval and timer wake with all three rail levels retained |
| PCF alarm and GPIO5 assertion | passed | AF asserted at 07:00:00, GPIO5 went low, and clearing AF released GPIO5 |
| GPIO5 RTC-domain pull-up | passed | alarm-driven EXT1 wake returned at the synthetic 07:00 boundary |
| one alarm, one RTC `Ext1` wake | passed | retained result reported the GPIO5 status bit and `alarm_was_pending=true` |
| release-firmware sleep implementation | passed | production entered deep sleep after its refresh and remained absent from USB for the bounded 45-second observation |
| passive image retention | passed | private images were provided and fulfill this requirement; the images are not published |
| PWR wake status | pending | one bounded GPIO18 `Ext1` observation is required |
| parent session and storage mode | pending | PWR hold, valid USB frame, RTC reset, and GPIO17 drop require device evidence |

USB disappearance alone remains insufficient evidence. The accepted wake proof
combines timed USB state with the RTC-retained `Ext1` status and alarm-flag
verdict. Manual battery-side measurement is out of scope. Follow the
[privacy and evidence rules](../privacy-and-evidence.md) before publishing
logs. Do not publish the supplied physical images.

[gpio-hold]: https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32s3/api-reference/peripherals/gpio.html
[hold-mask]: https://github.com/espressif/esp-idf/blob/v5.5.1/components/soc/esp32s3/gpio_periph.c
[esp-hal-5807]: https://github.com/esp-rs/esp-hal/pull/5807

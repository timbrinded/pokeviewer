# PCF85063 RTC and board-power bring-up

This procedure qualifies local civil time, the fixed daily alarm, and the V2
board's required power states. It is a diagnostic workflow, not the final
wake-refresh-sleep implementation.

## Fixed configuration

- PCF85063A at its fixed `0x51` address;
- I²C0 at 100 kHz on SDA GPIO47 and SCL GPIO48;
- RTC interrupt on GPIO5, active low;
- GPIO5 RTC-domain pull-up enabled and RTC-domain pull-down disabled for
  `Ext0` deep-sleep wake;
- GPIO17 high to hold the battery-controlled system path on;
- GPIO42 low while active and held low through deep sleep; and
- daily alarm at 07:00:00 local civil time.

Firmware talks directly to `0x51`; it does not select a device based on an I²C
scan. The ES8311 codec shares SDA/SCL and clamps both lines low when its rail is
off. Firmware therefore keeps that rail powered, applies the vendor ES8311
software-suspend sequence, and holds active-low GPIO42 low through deep sleep.
The audio rail remains powered; the ES8311 is software-suspended. Audio capture
and playback remain unconfigured. Wi-Fi, BLE, SD, touch, and environmental
sensors remain uninitialized.

## Automated coverage

The project-owned `Rtc` trait separates scheduling code from the physical
driver. Its deterministic fake covers:

- trustworthy reads and explicit oscillator-stop failure;
- valid and invalid set operations;
- leap-day, weekday, lower-year, and upper-year calendar cases; and
- alarm configuration, assertion, flag read, and flag clear behavior.

The PCF85063 adapter checks the oscillator-stop bit before returning a datetime.
Years outside 2000–2099 and invalid calendar values are rejected.

Run the host checks and the real target build:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
source .tools/export-esp.sh
CARGO_HOME="$PWD/.cargo-cache" \
  RUSTUP_HOME="$PWD/.rustup-cache" \
  cargo xtask firmware-diagnostic-build
```

## Device diagnostic

Provision a valid local datetime before running this diagnostic. Flash and
monitor using the pinned toolchain:

```sh
source .tools/export-esp.sh
cargo xtask firmware-diagnostic-flash
```

On boot, the combined hardware diagnostic:

1. holds GPIO17 high and GPIO42 low while the shared I²C bus is active;
2. rejects an RTC whose oscillator-stop bit is set;
3. reads the current datetime, writes that same value, and verifies readback;
4. reads and clears any pending alarm flag, verifies it stayed clear, and
   configures the daily 07:00 alarm; and
5. runs the panel patterns before switching off only the panel rail, then
   remains awake with the audio rail powered and the ES8311
   software-suspended.

The hardware diagnostic does not enter deep sleep. Use the separate
`sleep-diagnostic-build` and `sleep-diagnostic-flash` commands to verify the
GPIO5 RTC-domain pull-up, GPIO42 hold, deep-sleep entry, and `Ext0` wake path.

A successful sanitized log has this form:

```text
hardware diagnostics complete; RTC=YYYY-MM-DD HH:MM:SS; alarm_was_pending=false; panel rail off
```

Do not publish raw serial logs. Retain only the relevant lines after removing
device identifiers, user names, and home paths.

## Physical qualification

Run date readback at these boundaries without relying on the host's calendar:

| Case | Value | Expected weekday/result |
| --- | --- | --- |
| lower boundary | 2000-01-01 00:00:00 | Saturday, accepted |
| leap day | 2024-02-29 12:34:56 | Thursday, accepted |
| ordinary February | 2025-02-29 | rejected |
| year rollover | 2099-12-31 23:59:59 | Thursday, accepted |

Then configure a near-future alarm temporarily, observe GPIO5 transition low,
clear the alarm flag, and confirm GPIO5 returns high. Restore the fixed 07:00
alarm afterward.

Record power-control levels on USB and on a verified-polarity protected battery:

| Check | Required evidence | Status |
| --- | --- | --- |
| RTC presence | direct `0x51` set/read-back | USB smoke pass |
| datetime | sanitized boundary readback log | current-time read-back passed; boundary matrix pending |
| alarm | GPIO5 low/high trace plus flag log | pending |
| RTC wake bias | GPIO5 RTC-domain pull-up enabled, pull-down disabled | firmware path implemented; physical wake pending |
| battery latch | GPIO17 high measurement | pending |
| panel rail | GPIO6 low during refresh, high afterward | pending |
| audio rail | GPIO42 low while active and held low through sleep; codec software-suspended | firmware path passed; physical measurement pending |

USB-only smoke testing has flashed the connected V2 board, set and read back a
valid local datetime, rendered content-revision-1 daily-card CRC `d227338a`,
programmed the next 07:00 alarm, and entered deep sleep after the GPIO5 RTC-mux
cleanup; the USB connection disappeared as expected. That entry predates
qualification of the explicit RTC-domain GPIO5 pull-up and does not prove wake
reliability. A separate 2026-07-28 content-revision-2 smoke flash rendered CRC
`4f636e68` through the current awake-first firmware path; it does not alter the
deep-sleep qualification status. The scheduled RTC wake/reboot, boundary
values, GPIO5 alarm transition, rail measurements, battery polarity and
operation, and sanitized photographs remain pending. Follow the
[privacy and evidence rules](../privacy-and-evidence.md).

RTC wake and current measurements continue in the
[deep-sleep qualification](deep-sleep-qualification.md).

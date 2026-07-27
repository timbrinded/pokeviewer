# PCF85063 RTC and board-power bring-up

This procedure qualifies local civil time, the fixed daily alarm, and the V2
board's required power states. It is a diagnostic workflow, not the final
wake-refresh-sleep implementation.

## Fixed configuration

- PCF85063A at its fixed `0x51` address;
- I²C0 at 100 kHz on SDA GPIO47 and SCL GPIO48;
- RTC interrupt on GPIO5, active low;
- GPIO17 high to hold the battery-controlled system path on;
- GPIO42 high so the audio rail remains off; and
- daily alarm at 07:00:00 local civil time.

Firmware talks directly to `0x51`; it does not select a device based on an I²C
scan. Wi-Fi, BLE, audio, SD, touch, and environmental sensors remain
uninitialized.

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
  cargo xtask firmware-build
```

## Device diagnostic

Provision a valid local datetime before running this diagnostic. Flash and
monitor using the pinned toolchain:

```sh
source .tools/export-esp.sh
cargo xtask firmware-flash
```

On boot, the combined hardware diagnostic:

1. holds GPIO17 high and GPIO42 high;
2. rejects an RTC whose oscillator-stop bit is set;
3. reads the current datetime, writes that same value, and verifies readback;
4. reads and clears any pending alarm flag, verifies it stayed clear, and
   configures the daily 07:00 alarm; and
5. runs the panel patterns before switching its active-low rail off.

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
| RTC presence | sanitized I²C scan showing `0x51` | pending |
| datetime | sanitized boundary readback log | pending |
| alarm | GPIO5 low/high trace plus flag log | pending |
| battery latch | GPIO17 high measurement | pending |
| panel rail | GPIO6 low during refresh, high afterward | pending |
| audio rail | GPIO42 high throughout | pending |

Physical evidence is pending while the development user lacks permission to
open `/dev/ttyACM0`. Do not make the device node world-writable. Restore access
through the host's normal serial-device group, reconnect or re-login, and
follow the [privacy and evidence rules](../privacy-and-evidence.md).

RTC wake and current measurements continue in the
[deep-sleep qualification](deep-sleep-qualification.md).

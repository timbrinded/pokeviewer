# V2 release qualification procedure

- Status: blocked; power, failure-injection, and seven-day gates pending
- Delivery issue: [Q22 / #23][issue-23]
- Last reviewed: 2026-07-28

This procedure is the hardware release gate for the exact non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. Any failed threshold blocks release.

## Equipment and conditions

- Linux host with the repository-pinned Rust, `espup`, and `espflash`;
- direct data-capable USB cable;
- current instrument capable of resolving both e-paper peaks and below
  0.500 mA, with range, burden voltage, and sample rate recorded;
- current-limited bench supply or a protected 3.7 V single-cell battery;
- multimeter for rail levels; and
- camera with metadata removal available.

Record the firmware and `pokeviewerctl` full commits, artifact and content-pack
SHA-256, V2 marking, supply voltage/current limit, instrument model/range/sample
rate, and second-adult reviewer role. Never record a child's identity, home
detail, MAC, USB serial, credentials, raw device path, or unsanitized log.

## Clean-board setup

1. Disconnect the battery and USB. Inspect V2/non-touch markings and the panel
   connector.
2. Connect USB only. Confirm the host account has normal serial-group access;
   do not make the node world-writable.
3. Check out the exact commit with a clean worktree. Run the full local CI
   matrix and record artifact hashes.
4. Flash the release firmware with `cargo xtask firmware-flash`.
5. Use `pokeviewerctl info --device DEVICE`, `get-rtc`, and `diagnostics`.
   Sanitize outputs at capture time by replacing the device argument and
   omitting enumeration output.

## Observable checks and thresholds

| Requirement | Method | Pass threshold |
| --- | --- | --- |
| exact target | markings, contract probe, I²C population | V2, ESP32-S3-PICO-1-N8R8, 8 MB flash/PSRAM, `0x51` RTC, no `0x38` touch |
| RTC | set, read-back, oscillator-loss injection | exact read-back; invalid state shows `RTC`; no plausible card |
| USB | info/get/set/diagnostics | protocol v1 responses within two seconds; no wireless initialized |
| content/render | host matrix plus representative panel photos | all 151 pass; physical bytes/hash match expected golden |
| panel | full refresh and BUSY timing | completes once within 10 s; no clipping/ghosting; rail off afterward |
| active boot | release firmware over USB | one refresh, then USB disappears on deep-sleep entry within 30 s and does not re-enumerate before the alarm |
| rollover planning | host boundary tests plus synthetic near-07:00 diagnostic | exactly one boundary transition and a strictly future next alarm |
| daily wake | build/flash with `cargo xtask sleep-diagnostic-build` and `cargo xtask sleep-diagnostic-flash`; set to 06:59:30 and observe | GPIO5 RTC-domain pull-up enabled with pull-down disabled; prior card before 07:00; one new card at/after 07:00; one `Ext0` wake |
| reset/power loss | reset before/after 07:00; remove/restore power | correct display day and next alarm on every recovery |
| failure codes | `failure-diagnostic-flash` for RTC/panel/alarm | expected code/retained frame; at most one attempt; USB remains absent in no-wake deep sleep |
| active duration | current trace, boot to settled sleep | at most 30 s |
| deep sleep | battery-input current after 60 s settled | at most 0.500 mA |
| 72-hour sizing | repository calculator and intended cell | rated usable capacity is at least calculated minimum |

For panel/alarm fault injection, use a reversible test fixture and current
limit; never short a rail or connector. If an injection cannot be made safely,
record `FAIL` and block release rather than waiving it.

## Repeats, long run, and teardown

Run the pre-07:00 transition three times from a clean reset before the seven-day
qualification. The long run requires seven consecutive retained cards and
wake cycles, with no extra refresh. Afterward restore the correct local time,
disconnect the battery before changing wiring, remove temporary fixtures, and
confirm the repository and public evidence contain no private identifiers.

Copy [the evidence template][template] into a new ignored working directory,
replace every placeholder, and run:

```console
scripts/check-qualification-evidence.sh PATH
```

The validator rejects missing files, pending/failed checklist rows, non-full
commits or hashes, malformed/duplicate measurement and seven-day rows, failed
thresholds, the wrong board revision, and common path/device/credential leaks.
A structurally valid seven-day log must also match the existing Rust
qualification schedule generator exactly, including each date, weekday,
Pokémon, and framebuffer CRC.
A synthetic passing fixture and deliberate failure cases run in host CI; they
are validator tests, never physical qualification evidence.

## Capacity calculation

Use battery-input measurements, one event per day, and a justified usable
capacity fraction:

```console
scripts/calculate-battery-capacity.sh \
  SLEEP_MA ACTIVE_MA ACTIVE_SECONDS REFRESH_MA REFRESH_SECONDS USABLE_FRACTION
```

Illustrative arithmetic only—not a board measurement:

```console
scripts/calculate-battery-capacity.sh 0.100 80 2 120 5 0.80
```

This yields a minimum of `9.791 mAh` for those hypothetical inputs. The release
record must replace them with measured V2 values and compare the result with
the intended protected cell.

## Current dry-run result

The repository, host tests, artifact validation, templates, calculator, and
evidence validator are usable. The connected V2 board reports an ESP32-S3
revision v0.2 and 8 MB flash. Content-revision-2 firmware flashed and verified
over USB on 2026-07-28; protocol info and RTC set/read-back passed, and the
board rendered daily-card CRC `4f636e68`.

The corrected ESP-IDF-aligned implementation passed timer-only deep sleep, the
PCF alarm/GPIO5 assertion sequence, and one alarm-driven `Ext0` wake with the
alarm flag asserted. Production then refreshed once, entered deep sleep, and
did not re-enumerate during the bounded 45-second observation. Private physical
evidence also confirms a readable retained card while unplugged.

This is not release qualification. Battery-only operation, battery-side current
measurements, safe recovery injections, repeated 07:00 transitions, and the
seven-day run remain pending.

[issue-23]: https://github.com/timbrinded/pokeviewer/issues/23
[template]: ../evidence/qualification-template/

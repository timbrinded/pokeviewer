# V2 release qualification procedure

- Status: USB render/deep-sleep-entry smoke pass; wake and battery gates pending
- Delivery issue: [Q22 / #23][issue-23]
- Last reviewed: 2026-07-27

This procedure is the hardware release gate for the exact non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. Any failed threshold blocks release.

## Equipment and conditions

- Linux host with the repository-pinned Rust, `espup`, and `espflash`;
- direct data-capable USB cable;
- current instrument capable of resolving both e-paper peaks and below
  0.500 mA, with range, burden voltage, and sample rate recorded;
- current-limited bench supply or a protected 3.7 V single-cell battery whose
  connector polarity has passed the two-adult check;
- multimeter for connector polarity and rail levels; and
- camera with metadata removal available.

Record the firmware and `pokeviewerctl` full commits, artifact and content-pack
SHA-256, V2 marking, supply voltage/current limit, instrument model/range/sample
rate, and second-adult reviewer role. Never record a child's identity, home
detail, MAC, USB serial, credentials, raw device path, or unsanitized log.

## Clean-board setup

1. Disconnect the battery and USB. Inspect V2/non-touch markings and panel
   connector. Perform the [battery polarity gate](v2-board-contract.md#battery-connector-safety-gate).
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
| daily wake | build/flash with `cargo xtask sleep-diagnostic-build` and `cargo xtask sleep-diagnostic-flash`; set to 06:59:30 and observe | GPIO5 RTC-domain pull-up enabled with pull-down disabled; prior card before 07:00; one new card at/after 07:00; one `Ext0` wake |
| reset/power loss | reset before/after 07:00; remove/restore power | correct display day and next alarm on every recovery |
| failure codes | safe RTC/panel/alarm injections | expected code/flag; at most one attempt; terminal low-power state |
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
commit IDs, the wrong board revision, and common path/device/credential leaks.

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
revision v0.2 and 8 MB flash. Release firmware flashed and verified over USB;
protocol info and RTC set/read-back passed. After the GPIO5 RTC-mux cleanup, the
board rendered daily-card CRC `d227338a`, programmed the next alarm, and entered
deep sleep; the USB connection disappeared as expected. Device identifiers
were omitted and permissions were not weakened.

This is not release qualification. PSRAM identity, complete I²C population,
physical panel photographs, the scheduled RTC wake/reboot with the explicit
GPIO5 RTC-domain pull-up, alarm transitions, battery polarity, battery-only
operation, current measurements, recovery injections, repeated 07:00
transitions, and the seven-day run remain pending.

[issue-23]: https://github.com/timbrinded/pokeviewer/issues/23
[template]: ../evidence/qualification-template/

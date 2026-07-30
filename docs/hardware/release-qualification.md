# V2 release qualification procedure

- Status: blocked; remaining v1.1.0 device, failure-injection, and seven-day gates pending
- Delivery issue: [Q22 / #23][issue-23]
- Last reviewed: 2026-07-30

This procedure is the hardware release gate for the exact non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. Any failed threshold blocks release.

## Equipment and conditions

- Linux host with the bundled CLI and repository test tools;
- direct data-capable USB cable;
- the intended protected 3.7 V single-cell battery; and
- camera with metadata removal available.

Record the firmware and `pokeviewerctl` full commits, artifact and content-pack
SHA-256, V2 marking, supply condition, and second-adult reviewer role. Never
record a child's identity, home detail, MAC, USB serial, credentials, raw
device path, or unsanitized log.

## v1.1.0 qualification basis

The assembled device keeps its battery connected. Do not reflash it or change
its wiring for this qualification.

The installed device firmware came from commit
`2603f9314689a9ad075c344f79cb1fe9f74e2b19`. Record the release-candidate
commit in the release issue and candidate metadata. The firmware crates,
ESP32-S3 pad-hold crate, and generated content must have no difference between
the installed commit and the candidate commit.

The release evidence has two independent parts:

1. Qualify hardware behavior on the installed source-equivalent firmware.
2. Qualify the retained release archive in an isolated environment.

This procedure does not claim that the exact candidate bytes were flashed to,
or read back from, the device. Use the bundled `pokeviewerctl` for remaining
parent operations. Sanitize outputs at capture time by replacing the device
argument and omitting enumeration output.

## Observable checks and thresholds

| Requirement | Method | Pass threshold |
| --- | --- | --- |
| exact target | markings, contract probe, I²C population | V2, ESP32-S3-PICO-1-N8R8, 8 MB flash/PSRAM, `0x51` RTC, no `0x38` touch |
| RTC | set, read-back, oscillator-loss injection | exact read-back; invalid state shows `RTC`; no plausible card |
| USB | info/get/set/diagnostics/storage | startup handshake within six seconds; first parent command within 12 seconds; later responses within two seconds; no wireless initialized |
| content/render | host matrix plus generated PNGs and representative panel images | all 151 pass; normal, recharge, and unavailable battery states match expected goldens |
| panel | full refresh and BUSY timing | completes once within 10 s; no clipping/ghosting; rail off afterward |
| active boot | source-equivalent firmware over USB | one refresh, then USB disappears on deep-sleep entry within 30 s and does not re-enumerate before the alarm |
| rollover planning | host boundary tests plus synthetic near-07:00 diagnostic | exactly one boundary transition and a strictly future next alarm |
| daily wake | build/flash with `cargo xtask sleep-diagnostic-build` and `cargo xtask sleep-diagnostic-flash`; set to 06:59:30 and observe | GPIO5 RTC-domain pull-up enabled with pull-down disabled; prior card before 07:00; one new card at/after 07:00; one RTC `Ext1` wake with GPIO5 status |
| PWR tap | press and release PWR before three seconds | no panel refresh and no visible change |
| parent session | connect USB, start CLI with `--wait-for-device`, and hold PWR | continuous three-second hold plus valid framed request opens a two-minute session |
| simultaneous wake | hold PWR across a synthetic 07:00 alarm | daily refresh completes before parent-session evaluation |
| storage mode | use the confirmed CLI command in a parent session | response succeeds; `SET TIME` appears; RTC oscillator-stop flag verifies; GPIO17 drops; no ESP wake source remains |
| battery estimate | host curve/filter tests plus battery-only screen observations | 10 percent steps; low warning enters below 15 percent and clears at or above 20 percent; invalid sample shows `?%` |
| reset/RTC loss | reset before/after 07:00; storage mode | correct display day and next alarm after RTC setup |
| failure codes | host failure-policy tests and diagnostic builds | unique terminal policy and screen; every diagnostic image builds |

The v1.1.0 release accepts host failure-policy tests and deterministic
diagnostic builds in place of on-device RTC, panel, and alarm failure
diagnostics. Physical fault handling remains unverified. Do not disconnect the
panel, disconnect the battery, or flash diagnostic firmware on the assembled
qualification device.

## Repeats, long run, and teardown

Run the pre-07:00 transition three times from a clean reset before the
seven-day qualification. The long run requires seven consecutive retained
cards and wake cycles, with no extra refresh. Afterward restore the correct
local time. Do not change the device wiring. Confirm that the repository and
public evidence contain no private identifiers.

Copy [the evidence template][template] into a new ignored working directory,
replace every placeholder, and run:

```console
scripts/check-qualification-evidence.sh PATH
```

The validator rejects missing files, pending/failed checklist rows, non-full
commits or hashes, malformed or duplicate seven-day rows, failed thresholds,
the wrong board revision, and common path/device/credential leaks.
A structurally valid seven-day log must also match the existing Rust
qualification schedule generator exactly, including each date, weekday,
Pokémon, and framebuffer CRC.
A synthetic passing fixture and deliberate failure cases run in host CI; they
are validator tests, never physical qualification evidence.

## Battery scope

The v1.1.0 screen uses a generic LiPo open-circuit-voltage estimate. Manual
current measurement, discharge testing, runtime certification, charger
certification, and a universal capacity claim are out of scope. Qualification
must test the display states and hysteresis. It must not present the estimate
as a fuel gauge or safety control.

## Current dry-run result

The repository, host tests, artifact validation, templates, and evidence
validator are usable. The connected V2 board reports an ESP32-S3 revision v0.2
and 8 MB flash. Content-revision-2 firmware flashed and verified over USB on
2026-07-28; protocol info and RTC set/read-back passed, and the board rendered
daily-card CRC `4f636e68`.

The v1.0.0 ESP-IDF-aligned implementation passed timer-only deep sleep, the PCF
alarm/GPIO5 assertion sequence, and one alarm-driven `Ext0` wake with the alarm
flag asserted. Production then refreshed once, entered deep sleep, and did not
re-enumerate during the bounded 45-second observation. Private physical images
were provided and fulfill the readable retained-card requirement. The images
are not published.

Battery-only RTC operation and a synthetic 06:59 to 07:00 transition passed.
The card changed once at the scheduled alarm and remained visible. A later
overnight battery-only transition also passed. The battery screen showed a
plausible 10 percent step without the low-battery warning.

The v1.1.0 PWR-gated parent setup also passed on 2026-07-30. A continuous PWR
hold with an active `set-rtc --wait-for-device` command showed `SET TIME`,
returned the RTC read-back, restored the card, and entered normal operation.
A later PWR hold without an active framed command left the retained card
unchanged. Private physical images were provided and fulfill these
readable-display requirements. The images are not published.

The candidate from commit
`4ae630f419b1165c719108db3069ab880c6f5a64` passed an artifact-only rehearsal
in an isolated Ubuntu 24.04 environment with no network or source checkout.
Its outer checksum, internal checksums, CLI version, metadata contract, and
required release documents passed. This documentation change invalidates that
candidate. A replacement candidate from merged `main` must repeat the
artifact rehearsal. Exact on-device byte identity is not part of this
qualification.

This is not complete v1.1.0 release qualification. Storage mode, simultaneous
alarm and PWR wake, two more clean-reset synthetic 07:00 transitions, the
remaining seven-day run, restoration of the correct local time, and the
replacement candidate rehearsal remain pending.

[issue-23]: https://github.com/timbrinded/pokeviewer/issues/23
[template]: ../evidence/qualification-template/

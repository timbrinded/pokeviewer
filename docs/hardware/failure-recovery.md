# Bounded failure and recovery contract

- Status: implemented; hardware injection evidence pending
- Delivery issue: [Q20 / #21][issue-21]
- Last reviewed: 2026-07-28

Every expected failure has a stable adult-facing code, one wired diagnostic
bit, at most one automatic hardware attempt per wake, and a terminal action.

| Failure | Code | Flag | Attempts | Screen | Terminal recovery |
| --- | --- | ---: | ---: | --- | --- |
| invalid/stopped/unreadable RTC | `RTC` | `0x0001` | 0 | setup instructions | wired `pokeviewerctl set`, verified read-back, software restart |
| corrupt/incompatible pack | `PACK` | `0x0002` | 1 | `REFLASH` | no-wake deep sleep; external reset after reinstall |
| panel init/refresh/BUSY | `PANEL` | `0x0004` | 1 | prior frame retained | no-wake deep sleep; inspect, then external reset |
| daily alarm arm | `ALARM` | `0x0008` | 1 | `RESET` | no-wake deep sleep; external reset |
| unsupported wake source | `WAKE` | `0x0010` | 0 | `RESET` | no-wake deep sleep; external reset |

Invalid RTC exposes `RTC` through the bounded wired diagnostics command and
accepts only the versioned provisioning protocol when the RTC bus is available.
Every other terminal failure logs code, bit, attempt count, and rail state
once, retains GPIO6/GPIO17 high and GPIO42 low, and enters deep sleep with no
wake source. The ESP32-S3 remains asleep until external reset or power cycling;
there is no timer, RTC wake, automatic refresh, or application retry.

The panel adapter already bounds BUSY waits to 500 ten-millisecond polls. The
runtime invokes initialization/full refresh once. A panel failure cannot
reliably render its own diagnostic, so the prior e-paper frame remains visible.
Production also rejects wake sources other than cold/reset and RTC `Ext0`
before it can render a plausible daily card.

The no-wake terminal follows ESP-IDF's documented
[`esp_deep_sleep_start()` behavior][idf-no-wake]. The pinned ESP-HAL
`Rtc::sleep_deep(&[])` takes the same empty wake-source configuration.

## Fault-injection evidence

Host tests prove:

- invalid RTC renders only the `RTC` setup frame;
- a corrupt pack returns a content error without changing a white framebuffer;
- all policies use distinct flags and screens and allow no more than one
  automatic attempt;
- fixed recovery labels validate before framebuffer mutation; and
- the valid path still renders all 151 entries and reviewed daily goldens.

Three safe policy-injection images exercise the physical terminal without
disconnecting buses or shorting rails:

```console
cargo xtask failure-diagnostic-flash rtc
cargo xtask failure-diagnostic-flash panel
cargo xtask failure-diagnostic-flash alarm
```

Run one image at a time. `rtc` and `alarm` refresh their recovery screens;
`panel` deliberately preserves the prior frame. Each image logs one bounded
verdict, turns the panel rail off, and enters no-wake deep sleep. A passing
observation requires USB to disappear within 30 seconds, remain absent for a
bounded 60-second observation, and show `codec_suspended=true` in the captured
line before sleep.

The committed [recovery-screen goldens][screens] record exact framebuffer and
PNG hashes. Safe hardware injections for RTC, panel, and alarm plus an active
duration/power trace remain pending a safe fixture and battery-side
qualification.

[idf-no-wake]: https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32s3/api-reference/system/sleep_modes.html#entering-deep-sleep
[issue-21]: https://github.com/timbrinded/pokeviewer/issues/21
[screens]: ../evidence/recovery-screens/README.md

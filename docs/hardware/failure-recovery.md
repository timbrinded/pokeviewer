# Bounded failure and recovery contract

- Status: implemented; hardware injection evidence pending
- Delivery issue: [Q20 / #21][issue-21]
- Last reviewed: 2026-07-27

Every expected failure has a stable adult-facing code, one wired diagnostic
bit, at most one automatic hardware attempt per wake, and a terminal action.

| Failure | Code | Flag | Attempts | Screen | Terminal recovery |
| --- | --- | ---: | ---: | --- | --- |
| invalid/stopped/unreadable RTC | `RTC` | `0x0001` | 0 | setup instructions | wired `pokeviewerctl set`, verified read-back, software restart |
| corrupt/incompatible pack | `PACK` | `0x0002` | 1 | `REFLASH` | indefinite deep sleep, then install a verified release |
| panel init/refresh/BUSY | `PANEL` | `0x0004` | 1 | best effort only | indefinite deep sleep, inspect/reset |
| daily alarm arm | `ALARM` | `0x0008` | 1 | best effort only | indefinite deep sleep, inspect/reset |
| unsupported wake source | `WAKE` | `0x0010` | 0 | `RESET` | indefinite deep sleep, reset |

Invalid RTC is the only failure that intentionally remains awake: it exposes
`RTC` through the bounded wired diagnostics command and accepts only the
versioned provisioning protocol. All terminal failures log only code, bit,
attempt count, and rail state, then enter deep sleep with no automatic wake
source. Reset, external power cycling, or reflashing is therefore required;
there is no reboot, refresh, or retry loop.

The panel adapter already bounds BUSY waits to 500 ten-millisecond polls. The
runtime invokes initialization/full refresh once. Alarm configuration is
invoked once after a successful frame. A panel failure cannot reliably render
its own diagnostic; this limitation is explicit rather than pretending a
failed output path succeeded.

## Fault-injection evidence

Host tests prove:

- invalid RTC renders only the `RTC` setup frame;
- a corrupt pack returns a content error without changing a white framebuffer;
- all policies use distinct flags and screens and allow no more than one
  automatic attempt;
- fixed recovery labels validate before framebuffer mutation; and
- the valid path still renders all 151 entries and reviewed daily goldens.

The committed [recovery-screen goldens][screens] record exact framebuffer and
PNG hashes. Safe hardware injections for RTC, panel, and alarm plus an active
duration/power trace remain pending serial access.

[issue-21]: https://github.com/timbrinded/pokeviewer/issues/21
[screens]: ../evidence/recovery-screens/README.md

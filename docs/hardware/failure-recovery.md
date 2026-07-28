# Bounded failure and recovery contract

- Status: implemented; hardware injection evidence pending
- Delivery issue: [Q20 / #21][issue-21]
- Last reviewed: 2026-07-28

Every expected failure has a stable adult-facing code, one wired diagnostic
bit, at most one automatic hardware attempt per wake, and a terminal action.

| Failure | Code | Flag | Attempts | Screen | Terminal recovery |
| --- | --- | ---: | ---: | --- | --- |
| invalid/stopped/unreadable RTC | `RTC` | `0x0001` | 0 | setup instructions | wired `pokeviewerctl set`, verified read-back, software restart |
| corrupt/incompatible pack | `PACK` | `0x0002` | 1 | `REFLASH` | remain awake, then install a verified release |
| panel init/refresh/BUSY | `PANEL` | `0x0004` | 1 | best effort only | remain awake, inspect/reset |
| daily alarm arm | `ALARM` | `0x0008` | 1 | best effort only | diagnostic/final-runtime reservation; remain awake during staged bring-up |
| unsupported wake source | `WAKE` | `0x0010` | 0 | `RESET` | diagnostic/final-runtime reservation; remain awake during staged bring-up |

Invalid RTC exposes `RTC` through the bounded wired diagnostics command and
accepts only the versioned provisioning protocol. During awake-first bring-up,
all terminal failures log only code, bit, attempt count, and rail state, then
remain awake with no automatic reset, refresh, sleep, or application retry.
Reset, external power cycling, or reflashing is required.

The panel adapter already bounds BUSY waits to 500 ten-millisecond polls. The
runtime invokes initialization/full refresh once. Normal awake mode does not
configure the alarm; the dedicated sleep diagnostic retains that boundary. A
panel failure cannot reliably render its own diagnostic; this limitation is
explicit rather than pretending a failed output path succeeded.

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
duration/power trace remain pending a safe fixture and battery-side
qualification.

[issue-21]: https://github.com/timbrinded/pokeviewer/issues/21
[screens]: ../evidence/recovery-screens/README.md

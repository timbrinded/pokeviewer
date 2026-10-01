# Hardware validation and diagnostics

Device checks are optional. They are useful when a change affects the panel,
RTC, sleep, buttons, USB protocol, or battery state, and they are not a release
gate ([ADR 0008](../decisions/0008-publish-releases-from-one-verified-workflow.md)).
A documentation, workflow, or host-tool change needs no device check.

## Choose one bounded check

Run the automated tests first. Then make one observation of the changed
behavior and stop:

| Changed behavior | Sufficient device check |
| --- | --- |
| panel or card layout | boot once and confirm one correct full refresh |
| RTC or scheduled wake | set a time just before 07:00 and confirm one transition |
| sleep | confirm that the device sleeps and wakes once as intended |
| `PWR` session or USB protocol | run one waiting CLI command and hold `PWR` once |
| `BOOT` restart | confirm that a tap does nothing and a one-second hold flashes green and refreshes |
| battery state or USB report | compare one `get-battery` value with a multimeter reading at the cell terminals (within 150 mV), then confirm that a `PWR` session does not replace the 07:00 reading |

If a check fails, keep the smallest output that identifies the failure. Follow
the [privacy rules](../privacy-and-evidence.md).

## Diagnostic images

Each image is a separate binary in `pokeviewer-firmware`. CI builds all of
them. Flash one with `cargo xtask <name>-flash`; it builds the image, writes
it, and opens the serial monitor. Reflash the release firmware and restore the
real local time afterwards.

| Image (`cargo xtask …`) | What it does | Passing log line starts with |
| --- | --- | --- |
| `firmware-diagnostic-flash` | Shows white, black, checkerboard, border, and text frames; validates the RTC, rewrites and reads it back, and arms 07:00. Stays awake. Needs a valid RTC. | `hardware diagnostics complete` |
| `timer-sleep-diagnostic-flash` | Sleeps for ten seconds with the rails held and wakes on the timer. | `timer sleep diagnostic passed` |
| `rtc-alarm-assertion-diagnostic-flash` | Stays awake, waits for the PCF85063 alarm, and checks that the alarm flag pulls GPIO5 low and clearing it releases GPIO5. | `RTC alarm assertion diagnostic passed` |
| `sleep-diagnostic-flash` | Shows one frame, sleeps until the RTC alarm, then checks the EXT1 status bit and alarm flag and stays awake with the verdict. Set the RTC just before 07:00 first. | `sleep diagnostic passed` |
| `failure-diagnostic-flash rtc`, `panel`, or `alarm` | Injects one terminal failure, logs once, and sleeps with only `BOOT` armed. USB should disappear within 30 seconds. | `failure diagnostic; injected_code=` |
| `usb-provisioning-flash` | Serves the USB protocol while awake, without the display or sleep. Use it to test `pokeviewerctl`. | (no log line) |

Exactly one RTC wake must follow one alarm. A repeated `sleep diagnostic` wake
is a failure. USB disappearing on its own does not prove that the device slept
correctly.

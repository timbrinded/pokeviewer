# Optional hardware validation

Hardware validation is useful when a change affects the panel, RTC, sleep,
buttons, USB protocol, or battery display. It is not a release gate.

Run the automated tests first. Then use one bounded device check for the
changed behavior:

| Changed behavior | Sufficient device check |
| --- | --- |
| panel or card layout | boot once and confirm one correct full refresh |
| RTC or scheduled wake | set a near-boundary time and confirm one transition |
| sleep | confirm that the device enters sleep and wakes once as intended |
| PWR parent session | run one waiting CLI command and hold `PWR` once |
| battery display | confirm one plausible battery-only display state |

Stop after the intended observation. Do not add repeated resets, a seven-day
run, photographs, a second reviewer, private evidence templates, or an
isolated-host rehearsal.

If a check fails, keep the smallest sanitized output that identifies the
failure. Never publish a child or household detail, device identifier, private
path, credential, or raw log.

Flash only when the changed firmware must be installed. A documentation,
workflow, or host-tool-only change does not require a device flash.

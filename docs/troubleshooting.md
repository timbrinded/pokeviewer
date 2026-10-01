# Adult troubleshooting and recovery

Do not repeatedly reset a warm, damaged, wet, or swollen device. Disconnect
power if safe and follow the [safety guide](safety.md).

| Screen/code | Meaning | Adult action |
| --- | --- | --- |
| `RTC` setup | stopped, unreadable, or invalid clock | connect USB; run `set-rtc`; require matching read-back |
| `PACK` / `REFLASH` | compiled content is corrupt or incompatible | verify and cleanly reinstall the exact release |
| `PANEL` / `RESET` | panel init, BUSY, or full refresh failed | disconnect battery; inspect panel connector; hold `BOOT` for one second once |
| `ALARM` / `RESET` | next 07:00 alarm could not be armed | hold `BOOT` for one second once; if it returns, connect USB and inspect diagnostics |
| `WAKE` / `RESET` | unsupported wake source | hold `BOOT` for one second once; qualify wiring if repeated |

Terminal failures other than USB-capable RTC setup enter deep sleep after one
bounded attempt. They do not automatically reset, refresh, or retry. Only a
one-second `BOOT` hold wakes the device; `PWR` does nothing in this state. The
green light flashes once when the restart is accepted. A panel failure leaves
the prior card visible because the failed output path cannot reliably display
its own code, so an unresponsive `PWR` with an old card can also mean a panel
failure.

## Common setup failures

- `failed to open selected serial device`: reconnect USB, confirm the explicit
  path locally, and fix normal device-group membership. Do not use `chmod 666`.
- `timed out waiting for device response`: confirm release firmware is running,
  use a data-capable direct cable, and retry one command.
- invalid datetime: use local `YYYY-MM-DDTHH:MM:SS` with a real date in
  2000–2099.
- prior card before 07:00: expected passive display-day behavior, not a fault.
- old card after complete power loss: e-paper retention does not prove RTC
  validity; connect USB and read/set the clock.
- no touch response: expected; the supported SKU has no touch controller.
- no network setup: expected; v1 is fully offline.
- USB repeatedly disappears and returns after an attempted sleep: invalid sleep
  behavior, not proof of a valid wake; record the interval and return to the
  awake baseline.

If a fault repeats after one reset, stop. Preserve only sanitized codes and
hashes, then use the
[optional hardware validation guide](hardware/validation.md).

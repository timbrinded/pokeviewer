# USB provisioning protocol v1

- Status: accepted
- Delivery issue: [T16 / #17][issue-17]
- Transport: ESP32-S3 hardwired USB Serial/JTAG CDC-ACM
- Baud setting: 115,200, 8 data bits, no parity, one stop bit
- Maximum frame: 30 bytes

The protocol is used only while an adult explicitly connects the device over
USB. It does not initialize Wi-Fi or Bluetooth and contains no account,
telemetry, device identifier, MAC address, timezone, or host path.

## Frame

All multi-byte integers are unsigned little-endian:

| Offset | Size | Field |
| ---: | ---: | --- |
| 0 | 4 | ASCII magic `PKVW` |
| 4 | 1 | protocol version, exactly `1` |
| 5 | 1 | kind: `0` request, `1` response |
| 6 | 1 | command ID |
| 7 | 1 | payload length, `0`–`16` |
| 8 | 2 | host-selected request ID |
| 10 | 0–16 | payload |
| final 4 | 4 | CRC-32/ISO-HDLC over header and payload |

The firmware decoder owns one 30-byte buffer, resynchronizes on the magic
prefix, rejects invalid versions, lengths, kinds, commands, and checksums, and
discards a partial frame when the provisioning timeout expires. One transport
poll drains at most the USB endpoint's 64-byte packet.

## Commands and responses

Every response payload begins with one status byte: `0` success, `1` invalid
request, `2` unsupported command, or `3` device failure.

| ID | Command | Request payload | Successful response after status |
| ---: | --- | --- | --- |
| 1 | Handshake | empty | firmware major, minor, patch, capability bits |
| 2 | Read RTC | empty | seven local datetime bytes |
| 3 | Set RTC | seven local datetime bytes | seven read-back datetime bytes |
| 4 | Diagnostics | empty | 16-bit bounded diagnostic flags |
| 5 | Enter storage | empty | no additional bytes |
| 6 | Read battery | empty | state byte and cell millivolts as `u16` |

The datetime payload is `year:u16, month:u8, day:u8, hour:u8, minute:u8,
second:u8`. It is an explicit local wall clock with no UTC offset or daylight
saving rule. Firmware validates the complete Gregorian value and the RTC's
2000–2099 range before calling `set_datetime`; invalid input cannot partially
change RTC state.

The handshake capability bits are:

| Bit | Capability |
| ---: | --- |
| 0 | handshake |
| 1 | read RTC |
| 2 | set RTC |
| 3 | diagnostics |
| 4 | enter storage mode |
| 5 | read retained battery snapshot |

Commands 1 to 5 keep their existing wire IDs and response shapes. Firmware
reports product version `1.2.0` and capability mask `0x3f`. Older v1.1.0
firmware reports `0x1f` and does not implement command 6.

The storage command is accepted only in a PWR-gated parent session, which
already shows `SET TIME`. Firmware writes the successful response first. It
then sends the PCF85063 software-reset command, verifies the oscillator-stop
flag, waits 100 ms, drops GPIO17, and enters deep sleep with no ESP wake
source.

Diagnostic bits 0 to 4 are the failure flags in the
[runtime failure table](hardware/wake-sleep-state-machine.md#failure-path). Bit 5 means that
a valid retained scheduled battery snapshot is available. Bit 6 means that the
retained state is `Recharge`. The response remains 16 bits.

The read-battery response after the status byte is exactly three bytes:
`state:u8, cell_mv:u16`. State `0` is `Normal` and requires 3,750 mV through
4,500 mV. State `1` is `Recharge` and requires 2,500 mV through 3,849 mV.
Values from 3,750 mV through 3,849 mV are valid with either state because of
hysteresis. State `2` is `Unavailable` and requires `0` mV. Other lengths,
state values, and state/value combinations are invalid responses.

The response is the retained scheduled snapshot committed only by a validated
RTC alarm wake. The command does not take a new ADC sample. A PWR session,
reset, or invalid-RTC path cannot replace the snapshot.

## Linux CLI

`pokeviewerctl` is the host side. Build it with
`cargo build --release --locked -p pokeviewerctl`. Its commands:

```console
target/release/pokeviewerctl list
target/release/pokeviewerctl info --device /dev/ttyACM0
target/release/pokeviewerctl get-rtc --device /dev/ttyACM0
target/release/pokeviewerctl get-battery --device /dev/ttyACM0
target/release/pokeviewerctl set-rtc --device /dev/ttyACM0 \
  --datetime 2026-07-27T19:05:09
target/release/pokeviewerctl set-rtc --device /dev/ttyACM0 \
  --now --wait-for-device
target/release/pokeviewerctl diagnostics --device /dev/ttyACM0
target/release/pokeviewerctl enter-storage --device /dev/ttyACM0 \
  --confirm-time-loss --wait-for-device
```

Only `list` prints discovered paths. Other output contains protocol, firmware,
RTC, battery, or diagnostic values but not the selected path. Errors omit host
paths and USB serial identifiers.

Each invocation opens the device, sends a handshake, and then sends at most
one command. Timeouts depend on `--wait-for-device`:

| Step | Without `--wait-for-device` | With `--wait-for-device` |
| --- | --- | --- |
| open the path | once; a missing path fails | polls every 250 ms for up to 60 seconds; permission denial is retried for up to 2 seconds while udev applies the group |
| handshake | one attempt, 2-second timeout | retried every 500 ms for up to 6 seconds |
| command response | 2 seconds | 12 seconds, because the firmware first refreshes the `SET TIME` screen |

Argument errors fail immediately. The CLI rejects `enter-storage` locally if
capability bit 4 is absent and `get-battery` if bit 5 is absent. Successful
battery output is exactly one of these forms:

```text
battery_state=normal cell_mv=3920
battery_state=recharge cell_mv=3720
battery_state=unavailable cell_mv=unavailable
```

The CLI returns nonzero for transport, compatibility, framing, status,
calendar, or battery-payload failures.

On Linux the user must already have permission to open the selected TTY. The
CLI does not run privilege-changing commands.

## Tests

Host tests cover all six commands, valid frames, noise resynchronization,
corruption, truncation, unsupported versions, length bounds, invalid calendar
fields, battery states and bounds, old-firmware capability rejection, option
validation, and transport timeouts. The `no_std` firmware handler is tested
with a fake RTC, including the rule that an invalid date never changes the
RTC, the set/read-back response, and the storage-session gate.

[issue-17]: https://github.com/timbrinded/pokeviewer/issues/17

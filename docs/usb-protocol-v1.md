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

Commands 1 to 4 keep their v1 wire IDs and response shapes. Firmware reports
product version `1.1.0` and capability mask `0x1f`.

The storage command is accepted only in a PWR-gated parent session, which
already shows `SET TIME`. Firmware writes the successful response first. It
then sends the PCF85063 software-reset command, verifies the oscillator-stop
flag, waits 100 ms, drops GPIO17, and enters deep sleep with no ESP wake
source.

Diagnostic bits 0 to 4 keep their existing failure meanings. Bit 5 means that
the battery sample is plausible. Bit 6 means that the low-battery warning is
active. The response remains 16 bits.

## Linux CLI

Build and use the pinned Rust utility:

```console
cargo build --release --package pokeviewerctl
cargo xtask usb-provisioning-build
cargo xtask usb-provisioning-flash
target/release/pokeviewerctl list
target/release/pokeviewerctl info --device /dev/ttyACM0
target/release/pokeviewerctl get-rtc --device /dev/ttyACM0
target/release/pokeviewerctl set-rtc --device /dev/ttyACM0 \
  --datetime 2026-07-27T19:05:09
target/release/pokeviewerctl set-rtc --device /dev/ttyACM0 \
  --now --wait-for-device
target/release/pokeviewerctl diagnostics --device /dev/ttyACM0
target/release/pokeviewerctl enter-storage --device /dev/ttyACM0 \
  --confirm-time-loss --wait-for-device
```

Only `list` prints discovered paths, because device discovery is its explicit
purpose. Other successful output contains protocol, firmware, RTC, or bounded
diagnostic values but not the selected path. Errors intentionally omit host
paths and USB serial identifiers. With `--wait-for-device`, the CLI polls the
exact path every 250 ms for at most 60 seconds. After the path opens, it
retries the startup handshake every 500 ms for up to six seconds. It allows up
to 12 seconds for the first parent-session command because the firmware first
refreshes the `SET TIME` screen. Later commands use a two-second response
timeout. Permission and argument errors fail immediately. Each non-info
command completes a handshake first. The CLI rejects storage mode locally if
capability bit 4 is absent. It returns nonzero for transport, compatibility,
framing, status, or calendar failures.

On Linux the user must already have permission to open the selected TTY. The
repository does not run privilege-changing commands.

## Verification status

Host tests cover all five commands, valid frames, noise resynchronization,
corruption, truncation, unsupported versions, length bounds, invalid calendar
fields, option validation, and transport timeouts. The `no_std` firmware
handler is tested with the fake RTC, including the no-mutation invalid-date
rule, exact set/read-back response, and storage-session gate.

The v1.1.0 physical handshake, set/read-back, and PWR-gated parent-session
workflow passed on 2026-07-30. A PWR hold without an active framed CLI request
left the retained card unchanged. Private physical images were provided and
fulfill the readable-display requirement. The images are not published.
Storage-mode qualification remains pending. Public evidence must be sanitized
under the public evidence policy. CI builds the CLI and USB provisioning
firmware and checks the release-firmware size budget.

[issue-17]: https://github.com/timbrinded/pokeviewer/issues/17

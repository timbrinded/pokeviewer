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

The datetime payload is `year:u16, month:u8, day:u8, hour:u8, minute:u8,
second:u8`. It is an explicit local wall clock with no UTC offset or daylight
saving rule. Firmware validates the complete Gregorian value and the RTC's
2000–2099 range before calling `set_datetime`; invalid input cannot partially
change RTC state.

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
target/release/pokeviewerctl diagnostics --device /dev/ttyACM0
```

Only `list` prints discovered paths, because device discovery is its explicit
purpose. Other successful output contains protocol, firmware, RTC, or bounded
diagnostic values but not the selected path. Errors intentionally omit host
paths and USB serial identifiers. The CLI uses a two-second read/write timeout
and returns nonzero for transport, compatibility, framing, status, or calendar
failures.

On Linux the user must already have permission to open the selected TTY. The
repository does not run privilege-changing commands.

## Verification status

Host tests cover valid frames, noise resynchronization, corruption, truncation,
unsupported versions, length bounds, invalid calendar fields, and transport
timeouts. The `no_std` firmware handler is tested with the fake RTC, including
the no-mutation invalid-date rule and exact set/read-back response.

The physical handshake/set/read-back transcript is pending device access; it
must not be fabricated and must be sanitized under the public evidence policy.

Validated release-build sizes on Linux x86-64 were:

| Artifact | Linked sections | File bytes |
| --- | ---: | ---: |
| `pokeviewerctl` | 337,802 bytes | 2,382,936 |
| USB provisioning firmware | 8,164 bytes | 1,239,784 |

The ELF file sizes include debug information required by the release profile;
linked-section size is the relevant on-device footprint.

[issue-17]: https://github.com/timbrinded/pokeviewer/issues/17

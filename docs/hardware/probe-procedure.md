# Sanitized V2 hardware probe

This procedure supplies the missing physical evidence for
[H02 / #3](https://github.com/timbrinded/pokeviewer/issues/3). Run it before
flashing application firmware or connecting a battery.

## Preconditions

- The board visibly carries the V2 marking.
- No battery is connected.
- USB is connected directly to a trusted Linux host.
- The current user has read/write access to the Espressif USB serial device
  through the host's normal device group or a narrowly scoped udev rule.
- `espflash` is the repository-pinned version.

Do not use `chmod 666`, run an unknown binary as root, or publish the raw device
path, MAC address, or USB serial.

## Photos

Capture tightly cropped front and rear photographs that show:

- product/SKU markings;
- the V2 silkscreen;
- the ESP32-S3 package marking;
- the e-paper connector/panel identity where visible; and
- the battery connector before a cell is attached.

Remove image metadata and inspect the background before publication.

## Chip and memory probe

Run the repository diagnostic command once it exists:

```console
cargo xtask hardware-probe --port /dev/ttyACM0 --redact
```

The sanitized report must contain only:

```text
board_revision=V2
chip=ESP32-S3
package=ESP32-S3-PICO-1-N8R8
flash_bytes=8388608
psram_bytes=8388608
usb_vid_pid=303a:1001
i2c_addresses=0x18,0x51,0x70
touch_controller=absent
```

The probe must fail rather than print a misleading success if chip, flash,
PSRAM, RTC, or touch-population checks disagree with the board contract.

## Battery polarity

Follow the multimeter procedure in the
[board contract](v2-board-contract.md#battery-connector-safety-gate). Record a
cropped orientation photo and a pass/fail statement; do not record a household
location or unrelated device identifier.

## Evidence review

Before committing any result:

1. compare the measured values with the V2 contract;
2. redact MAC and USB serial identifiers at source;
3. remove photograph metadata;
4. make the firmware commit and probe-tool version explicit; and
5. have another adult check connector polarity before a cell is inserted.

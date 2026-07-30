# Pokeviewer

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display.
It supports only the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 board.

The device shows the weekday, a Pokémon Yellow sprite, the English name, and
the canonical type or types. The device does not use Wi-Fi or BLE.

## Quick start

These instructions install Pokeviewer v1.1.0 from the official
[Pokeviewer releases](https://github.com/timbrinded/pokeviewer/releases) page.
They require an x86-64 Linux computer.

The ESP32-S3 ROM contains the factory download bootloader. This procedure
writes Pokeviewer firmware to flash memory.

### 1. Prepare the equipment

Get these items:

- the supported V2 board;
- a data-capable USB cable;
- a compatible protected battery; and
- an x86-64 Linux computer with `curl`, `tar`, `sha256sum`, and Cargo.

Read the [safety guide](docs/safety.md) before you connect the battery.

Install the pinned flash utility:

```console
cargo install espflash --version 4.5.0 --locked
espflash --version
```

The second command must report `espflash 4.5.0`.

### 2. Download and verify the release

Open a terminal.

Run these commands:

```console
mkdir pokeviewer-v1.1.0-install
cd pokeviewer-v1.1.0-install

curl --fail --location --remote-name \
  https://github.com/timbrinded/pokeviewer/releases/download/v1.1.0/pokeviewer-v1.1.0.tar.gz
curl --fail --location --remote-name \
  https://github.com/timbrinded/pokeviewer/releases/download/v1.1.0/pokeviewer-v1.1.0.tar.gz.sha256

sha256sum --check pokeviewer-v1.1.0.tar.gz.sha256
tar -xzf pokeviewer-v1.1.0.tar.gz
cd pokeviewer-v1.1.0
sha256sum --check SHA256SUMS
```

Stop if a checksum command reports a failure.

### 3. Start download mode

1. Disconnect the battery.
2. Disconnect the USB cable.
3. Press and hold the `BOOT` button.
4. Connect the USB cable.
5. Continue to hold `BOOT` for two seconds.
6. Release `BOOT`.

Find the serial device:

```console
ls /dev/ttyACM*
```

Set `DEVICE` to the path that the command shows:

```console
export DEVICE=/dev/ttyACM0
```

On Arch Linux, the serial-device group is usually `uucp`.
On Debian and Ubuntu, the group is usually `dialout`.

If Linux denies access, add your account to the applicable group.

Use one of these commands:

```console
# Arch Linux
sudo usermod --append --groups uucp "$USER"

# Debian or Ubuntu
sudo usermod --append --groups dialout "$USER"
```

Run only the command for your Linux distribution.
Then sign out and sign in.

Do not run the flash commands with `sudo`.
Do not make the serial device world-writable.

### 4. Flash the firmware

Confirm that `espflash` detects an ESP32-S3:

```console
espflash board-info \
  --chip esp32s3 \
  --port "$DEVICE" \
  --before no-reset \
  --after no-reset
```

Erase the flash memory:

```console
espflash erase-flash \
  --chip esp32s3 \
  --port "$DEVICE" \
  --before no-reset \
  --after no-reset
```

Write the release firmware:

```console
espflash write-bin \
  --chip esp32s3 \
  --port "$DEVICE" \
  --before no-reset \
  --after no-reset \
  0x0 pokeviewer-v1.1.0-esp32s3-v2.bin
```

Wait for the command to report a successful write.

### 5. Start the firmware

1. Disconnect the USB cable.
2. Keep the battery disconnected.
3. Wait ten seconds.
4. Make sure that you do not press `BOOT`.
5. Connect the USB cable normally.
6. Wait for the screen to show `SET TIME`.

This power cycle stops download mode and starts the installed firmware.

### 6. Set the local time

Set the command path:

```console
export CLI=./pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu
chmod u+x "$CLI"
```

If the serial path changed, set `DEVICE` to the new path.

Confirm communication with the device:

```console
"$CLI" info --device "$DEVICE"
```

Make sure that the Linux computer shows the correct local time.

Choose one time command.

To use the Linux local time, run:

```console
"$CLI" set-rtc \
  --device "$DEVICE" \
  --now
```

To use a test time, replace the example value:

```console
"$CLI" set-rtc \
  --device "$DEVICE" \
  --datetime 2030-01-02T06:59:30
```

Use the `YYYY-MM-DDTHH:MM:SS` format.
Do not add a time-zone suffix.

The command reads the RTC after the write.
The firmware then restarts, updates the display, and enters deep sleep.
The USB command interface stops after a successful time update.

Connect the battery before you disconnect the USB cable.

## Wake USB with the battery connected

When the firmware is in deep sleep, `ls /dev/ttyACM*` finds no board. This is
expected. Connect USB and start:

```console
"$CLI" info --device "$DEVICE" --wait-for-device
```

Press and hold `PWR` for at least three seconds, then release it. The command
waits for the device, verifies the firmware, and opens a two-minute parent USB
session.

## Change the time later

You do not have to flash the firmware again. Keep the battery connected.

1. Connect a data-capable USB cable.
2. Start this command:

   ```console
   "$CLI" set-rtc \
     --device "$DEVICE" \
     --now \
     --wait-for-device
   ```

3. Press and hold `PWR` for at least three seconds.
4. Release `PWR` when the screen shows `SET TIME`.
5. Wait for the command to show the RTC read-back.

The command waits for the exact device path for up to 60 seconds. It also
allows time for the `SET TIME` screen to refresh before it sends the time.
A `PWR` press or hold without an active command has no visible effect. The
`BOOT` button is for service and flashing only.

## Prepare the device for storage

Storage mode clears the RTC. The next start shows `SET TIME`.

1. Connect a data-capable USB cable.
2. Start this command:

   ```console
   "$CLI" enter-storage \
     --device "$DEVICE" \
     --confirm-time-loss \
     --wait-for-device
   ```

3. Press and hold `PWR` for at least three seconds.
4. Release `PWR` when the screen shows `SET TIME`.
5. Wait for the command to confirm storage mode.

The firmware then drops the power latch. Disconnect USB to complete the
power-off. A later `PWR` press starts the device.

## Normal operation

At 07:00 local time, the device wakes and shows the card for the new day.
The device then enters deep sleep. The e-paper panel keeps the card visible
without panel power.

The firmware contains all 151 Generation I entries.
The device does not require an account, an SD card, or internet access.
The supported board does not have a touchscreen.

The top corner shows a coarse battery estimate in 10 percent steps. The
estimate is not a fuel gauge. At a low estimate, the screen also shows the
lightning icon and `CHARGE!`. If the ADC reading is not plausible, the screen
shows `?%`. The board has no dedicated USB-power sense input. USB operation
with no battery can therefore show `100%`.

## Important notices

Pokeviewer is an unofficial, non-commercial fan project.
Nintendo, Creatures Inc., GAME FREAK inc., and The Pokémon Company
International do not endorse or sponsor this project.

The MIT license covers only the original source code.
It does not cover Pokémon names, characters, artwork, sprites, or related
media. Read [Third-party notices](THIRD_PARTY_NOTICES.md) before you
redistribute a build or an asset pack.

The development board is not a certified children's toy.
An adult must assemble, inspect, charge, and supervise the device.

## Documentation

- [Setup and operation](docs/user-guide.md)
- [Safety](docs/safety.md)
- [Troubleshooting and recovery](docs/troubleshooting.md)
- [Release verification](docs/release-verification.md)
- [Product contract](docs/product-contract.md)
- [Architecture decisions](docs/decisions/README.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)

## Local development

For local development information, read
[CONTRIBUTING.md](CONTRIBUTING.md).

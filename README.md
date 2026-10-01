# Pokeviewer

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display.
It supports only the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 board.

The device shows the weekday, a Pokémon Yellow sprite, the English name, and
the canonical type or types. The device does not use Wi-Fi or BLE.

## Quick start

These instructions install Pokeviewer v1.2.0 from the official
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
mkdir pokeviewer-v1.2.0-install
cd pokeviewer-v1.2.0-install

curl --fail --location --remote-name \
  https://github.com/timbrinded/pokeviewer/releases/download/v1.2.0/pokeviewer-v1.2.0.tar.gz
curl --fail --location --remote-name \
  https://github.com/timbrinded/pokeviewer/releases/download/v1.2.0/pokeviewer-v1.2.0.tar.gz.sha256

sha256sum --check pokeviewer-v1.2.0.tar.gz.sha256
tar -xzf pokeviewer-v1.2.0.tar.gz
cd pokeviewer-v1.2.0
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

**Troubleshooting: battery connected.** If `ls` finds no board, the installed
firmware is in deep sleep. Prepare the bundled CLI and wait for the device:

```console
chmod u+x ./pokeviewerctl-v1.2.0-x86_64-unknown-linux-gnu
./pokeviewerctl-v1.2.0-x86_64-unknown-linux-gnu info \
  --device "$DEVICE" \
  --wait-for-device
```

Hold `PWR` for at least three seconds, then release it. The path returns for a
two-minute parent USB session. This wakes the installed firmware; it does not
start download mode.

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
  0x0 pokeviewer-v1.2.0-esp32s3-v2.bin
```

Wait for the command to report a successful write.

### 5. Start the firmware

1. Disconnect the USB cable.
2. Keep the battery disconnected.
3. Wait ten seconds.
4. Make sure that you do not press `BOOT`.
5. Connect the USB cable normally.
6. Wait for the screen to show `SET TIME`. The green light turns on.

This power cycle stops download mode and starts the installed firmware.
The device waits two minutes for the time. If the green light turns off
first, hold `BOOT` for one second to start another two-minute wait.

### 6. Set the local time

Set the command path:

```console
export CLI=./pokeviewerctl-v1.2.0-x86_64-unknown-linux-gnu
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

3. Press and hold `PWR`. After three seconds, the green light turns on.
4. Release `PWR` when the screen shows `SET TIME`.
5. Wait for the command to show the RTC read-back.

The command waits for the exact device path for up to 60 seconds. It also
allows time for the `SET TIME` screen to refresh before it sends the time.
If no command is waiting, the green light turns off after 15 seconds and the
screen does not change. [Controls and lights](#controls-and-lights) describes
every button and light.

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

3. Press and hold `PWR`. After three seconds, the green light turns on.
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

The card shows one battery state. `Normal` shows no battery text or icon.
`Recharge` shows the lightning icon and `CHARGE!` at the bottom.
`Unavailable` shows `BAT ?` in the top-right corner. If an invalid scheduled observation follows `Recharge`, the complete
prior recharge snapshot remains. Otherwise, it commits `Unavailable` with
`0` mV. The display does not show battery percentage or make a precise
capacity claim.

The USB CLI reports the retained scheduled sample without changing the
display:

```console
"$CLI" get-battery --device "$DEVICE" --wait-for-device
```

Start the command, then press and hold `PWR` until the green light turns on.
Release `PWR` when `SET TIME` appears.

It reports the state and bounded cell millivolts. Millivolts are diagnostic
data, not a state-of-charge measurement. The board has no dedicated USB-power
sense input, so a USB-powered reading does not identify the cell's capacity.

## Controls and lights

The device has two buttons, `PWR` and `BOOT`, and one light that can show
green or orange. A child does not need to use them. A short press of either
button does not change the screen.

### `PWR` button

| What you do | What happens |
| --- | --- |
| Press briefly | Nothing visible. The device stays asleep. |
| Hold for three seconds | The green light turns on. The device listens for a computer for 15 seconds. |
| Hold for three seconds while a `pokeviewerctl` command waits | The screen shows `SET TIME`. The command runs. The green light stays on for up to two minutes, then the day's card returns. |
| Press when the device is off after storage mode | The device starts and shows `SET TIME`. |

`PWR` does nothing while the screen shows `POKEVIEWER ERROR`. Use `BOOT`.

### `BOOT` button

| What you do | What happens |
| --- | --- |
| Press briefly | Nothing visible. |
| Hold for one second | The green light flashes once. The device restarts, reads the clock, and redraws the screen. |
| Hold while you connect USB with the battery disconnected | The device enters flashing mode. The screen does not change. Use this only to [flash the firmware](#4-flash-the-firmware). |

Hold `BOOT` for one second after any error screen that says `RESET`, or if
the screen looks wrong. A restart redraws the current day's card. It does not
change the clock or the battery state.

### Lights

| Light | Meaning |
| --- | --- |
| Green flashes once | The device accepted a `BOOT` restart. |
| Green stays on | The device is listening for, or connected to, a computer. |
| Green off | Normal. The device is asleep or working without a computer. |
| Orange on | The battery is charging from USB. The charger controls this light. |
| Orange off | The battery is not charging, or it is full. |

### Screen messages

| Screen | Meaning | What to do |
| --- | --- | --- |
| Weekday, Pokémon, name, and type | Normal daily card. | Nothing. It changes at 07:00. |
| `BAT ?` in the top-right corner | The last scheduled battery reading was not valid. | Charge the battery. The next 07:00 reading replaces it. |
| Lightning icon and `CHARGE!` at the bottom | The battery is low. | Charge the battery. |
| `SET TIME` | The clock is not set, or a computer session is open. | [Change the time](#change-the-time-later). |
| `POKEVIEWER ERROR` with `RESET` | A bounded failure stopped the device. | Hold `BOOT` for one second. If it returns, read [troubleshooting](docs/troubleshooting.md). |
| `POKEVIEWER ERROR` with `REFLASH` | The firmware content is damaged. | [Flash the firmware](#4-flash-the-firmware) again. |

After a new clock setting or battery connection, the card can show `BAT ?`
until the first 07:00 update. Only the scheduled 07:00 wake measures the
battery.

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

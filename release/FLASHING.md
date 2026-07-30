# Flash Pokeviewer v1.1.0

This image supports only the non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. Read `SAFETY.md` before connecting a battery.

Install the pinned `espflash` 4.5.0 utility from its official release.

Disconnect the battery before you flash the board. Then start download mode:

1. Disconnect USB.
2. Press and hold `BOOT`.
3. Connect a data-capable USB cable.
4. Continue to hold `BOOT` for two seconds.
5. Release `BOOT`.

Identify the local serial path:

```console
espflash board-info --chip esp32s3 --port DEVICE \
  --before no-reset --after no-reset
```

Erase the board, then write the single merged image at offset `0x0`:

```console
espflash erase-flash --chip esp32s3 --port DEVICE \
  --before no-reset --after no-reset
espflash write-bin --chip esp32s3 --port DEVICE \
  --before no-reset --after no-reset \
  0x0 pokeviewer-v1.1.0-esp32s3-v2.bin
```

Disconnect USB for ten seconds. Do not press `BOOT`. Connect USB normally and
wait for `SET TIME`.

Provision the local RTC:

```console
chmod u+x pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu
./pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu --version
./pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu info --device DEVICE
./pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu set-rtc \
  --device DEVICE --now
```

The set command reads the RTC after the write. The device then renders the
daily card and enters deep sleep. Connect the battery before you disconnect
USB.

The image is intentionally fully offline. It refreshes at 07:00 local time
and otherwise retains the e-paper card without panel power. To change the time
later, connect USB, start the command with `--wait-for-device`, and hold `PWR`
for three seconds. You do not have to flash the firmware again.

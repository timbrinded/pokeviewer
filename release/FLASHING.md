# Flash Pokeviewer v1.0.0

This image supports only the non-touch Waveshare
ESP32-S3-ePaper-1.54-EN V2 board. Read `SAFETY.md` before connecting a battery.

Install the pinned `espflash` 4.5.0 utility from its official release, connect
the board with a data-capable USB cable, and identify the local serial path:

```console
espflash board-info --port DEVICE
```

Erase the board, then write the single merged image at offset `0x0`:

```console
espflash erase-flash --chip esp32s3 --port DEVICE
espflash write-bin --chip esp32s3 --port DEVICE \
  0x0 pokeviewer-v1.0.0-esp32s3-v2.bin
```

Keep USB attached and provision the local RTC:

```console
chmod u+x pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu
./pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu --version
./pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu info --device DEVICE
./pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu set-rtc \
  --device DEVICE --datetime YYYY-MM-DDTHH:MM:SS
./pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu get-rtc --device DEVICE
```

The image is intentionally fully offline. It refreshes at 07:00 local time
and otherwise retains the e-paper card without panel power.

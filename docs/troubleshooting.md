# Troubleshooting and recovery

Do not restart a device that is warm, damaged, wet, or swollen. Disconnect
power if it is safe to do so and follow the [safety guide](safety.md).

## Screens

An error screen shows `POKEVIEWER ERROR`, a code, and an action.

| Screen | Meaning | What to do |
| --- | --- | --- |
| `SET TIME` | The clock is stopped, unreadable, or invalid, or a computer session is open. | [Set the time](../README.md#change-the-time-later). If the clock is unset and the green light is off, hold `BOOT` for one second to start another two-minute wait. |
| `PACK` / `REFLASH` | The compiled content is corrupt or incompatible. | Verify the release again and reflash it. |
| `ALARM` / `RESET` | The next 07:00 alarm could not be set. | Hold `BOOT` for one second. If it returns, check the clock with `get-rtc`. |
| `WAKE` / `RESET` | The device woke for an unexpected reason. | Hold `BOOT` for one second. |
| Old card, `PWR` does nothing | Possible panel failure. A failed panel cannot show its own error. | Disconnect the battery, check the panel connector, reconnect, and hold `BOOT` for one second. |

After an error screen, the device sleeps until `BOOT` is held for one second.
It does not retry by itself, and `PWR` does nothing. The green light flashes
once when the restart is accepted. The restart runs the failed step once more.

If the same error returns after one restart, stop and report it in a GitHub
issue with the screen code. Do not include photographs of a child, your home,
or full device logs.

## Clock lost after the battery ran flat

If the battery runs completely flat, the clock can stop. The last card stays
visible on the e-paper, but the device will not change it. Charge the battery
under supervision, then set the time as described in the
[README](../README.md#change-the-time-later).

## Command errors

- `permission denied for selected serial device` or
  `failed to open selected serial device`: reconnect USB, check the device
  path, and add your account to the serial-device group as described in the
  README. Do not use `sudo` or `chmod 666`.
- `timed out waiting for selected serial device`: the device path did not
  appear within 60 seconds. Start the command again, then hold `PWR` for three
  seconds within that time.
- `timed out waiting for device response`: use a data-capable cable connected
  directly to the computer and run the command once more.
- Invalid datetime: use local `YYYY-MM-DDTHH:MM:SS` with a real date from 2000
  to 2099.

## Expected behavior

- The previous day's card stays visible until 07:00.
- The board has no touchscreen and no network setup.
- A short press of `PWR` or `BOOT` does not change the screen.

## Clean reinstall

Follow the [quick start](../README.md#quick-start) from step 2: verify the
release, disconnect the battery, flash, start the firmware, and set the time.
Connect the battery before you disconnect USB.

The device has no over-the-air update.

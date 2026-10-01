# Pokeviewer v2.0.0

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board. It shows
the weekday, a Pokémon Yellow sprite, the English name, and the canonical type
or types, and changes the card at 07:00 local time. It has no Wi-Fi, BLE,
account, SD card, or over-the-air update.

This is a major release: the battery display, the button behaviour, and the
recovery steps change from v1.1.0.

## Battery

- The percentage is gone. The card shows one of three states: `Normal` (no
  battery text), `Recharge` (lightning icon and `CHARGE!` at the bottom), or
  `Unavailable` (`BAT ?` in the top-right corner). `Recharge` starts below
  3,750 mV and clears at or above 3,850 mV.
- The battery is measured every three hours, at 07:00, and on a `BOOT`
  restart. The card redraws only when the state changes, so `CHARGE!` clears
  within three hours of charging, or at once after a `BOOT` restart.
- The orange charger light is the reliable sign that charging has finished. A
  reading taken while USB is connected sees the charger's voltage.
- New `pokeviewerctl get-battery` reports the retained state and cell
  millivolts. Millivolts are diagnostic data, not a capacity measurement.

## Buttons and lights

- `PWR`: a short press does nothing visible. A three-second hold turns the
  green light on and opens the parent session for `pokeviewerctl`.
- `BOOT`: a one-second hold now restarts the firmware and redraws the screen,
  including after any error screen. Recovery no longer requires opening the
  device or disconnecting the battery. Holding `BOOT` at power-up with the
  battery disconnected still enters flashing mode.
- Green light: steady while the device listens for or serves a computer; one
  flash when a `BOOT` restart is accepted.

## Power and reliability

- The unused SHTC3 sensor is put to sleep at every boot, removing about 45 µA
  of idle current.
- The device no longer stays awake indefinitely when a button or the RTC line
  is held low at sleep entry. A stuck RTC interrupt is reported as `ALARM`.
- `pokeviewerctl --wait-for-device` tolerates the short moment when a new
  serial device has not yet received its permissions.

## Upgrading

Follow the README quick start. Flashing does not change the RTC, so the time
is kept. Use the v2.0.0 `pokeviewerctl` with v2.0.0 firmware.

## Contents

One merged firmware image flashable at offset `0x0`, one Linux x86-64
`pokeviewerctl`, the compiled offline content pack, SHA-256 checksums, and
setup, safety, and troubleshooting documentation.

Pokeviewer is an adult-built, child-adjacent development-board project, not a
certified finished toy. No board, enclosure, battery, charger, or cable is
included. Battery runtime is not guaranteed. Read the safety guide before you
connect a protected single-cell battery.

This is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

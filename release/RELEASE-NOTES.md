# Pokeviewer v2.1.0

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board. It shows
the weekday, a Pokémon Crystal sprite, the English name, and the canonical type
or types, and changes the card at 07:00 local time. It has no Wi-Fi, BLE,
account, SD card, or over-the-air update.

## Pokémon

- The device now shows all 251 Generation I and II Pokémon, National Pokédex
  IDs 1 to 251.
- Sprites come from Pokémon Crystal instead of Pokémon Yellow. Each sprite
  pixel is drawn as one of four shades (white, light, dark, or black) using
  black-and-white dot patterns, with the normal full refresh. Outlines stay
  solid black, and large black bodies keep their detail.
- The daily order is new. Every Pokémon appears once in each 251-day cycle,
  and any seven consecutive days show Pokémon at least 31 Pokédex numbers
  apart. The cycle still counts from 2026-01-01 and still changes at 07:00.

## Upgrading

Follow the README quick start. Flashing does not change the RTC, so the time
is kept. After flashing, the card shows the Pokémon the new order assigns to
the current day, which can differ from the one v2.0.0 showed. Use the v2.1.0
`pokeviewerctl` with v2.1.0 firmware. The USB protocol, buttons, lights, and
battery display are unchanged from v2.0.0.

## Contents

One merged firmware image flashable at offset `0x0`, one Linux x86-64
`pokeviewerctl`, the compiled offline content pack (`pokeviewer-v2.pack`),
SHA-256 checksums, and setup, safety, and troubleshooting documentation.

Pokeviewer is an adult-built, child-adjacent development-board project, not a
certified finished toy. No board, enclosure, battery, charger, or cable is
included. Battery runtime is not guaranteed. Read the safety guide before you
connect a protected single-cell battery.

This is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

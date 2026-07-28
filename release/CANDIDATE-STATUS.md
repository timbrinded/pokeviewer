# Pokeviewer v1.0.0 candidate status

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board.

The display shows the weekday, a Pokémon Yellow sprite, English name, and
canonical type or types. The selection changes deterministically at 07:00
local time. There is no runtime internet, Wi-Fi, BLE, touch, account, location,
SD-card, or over-the-air update dependency.

This is an adult-built, child-adjacent development-board project, not a
certified finished toy. No board, enclosure, battery, charger, or cable is
included. Battery runtime depends on the measured assembled device. Complete
the safety guidance before connecting a protected single-cell battery.

Complete RTC power loss requires an adult to reconnect USB and set the local
clock again. Read `USER-GUIDE.md`, `SAFETY.md`, and `TROUBLESHOOTING.md` before
use.

This candidate is not physically qualified or approved for publication until
the seven-day battery run and exact-artifact clean-host rehearsal are complete.
The current awake-first firmware is a development baseline and is not the v1
release candidate: stable deep sleep and RTC wake remain unqualified.
It is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

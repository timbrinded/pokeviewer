# Pokeviewer v1.0.0

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board.

The display shows the weekday, a Pokémon Yellow sprite, English name, and
canonical type or types. The selection changes deterministically at 07:00
local time. There is no runtime internet, Wi-Fi, BLE, touch, account, location,
SD-card, or over-the-air update dependency.

This release contains one merged firmware image flashable at offset `0x0`, one
Linux x86-64 provisioning CLI, the compiled offline content pack, SHA-256
checksums, and adult setup, safety, and troubleshooting documentation.

Complete RTC power loss requires an adult to reconnect USB and set the local
clock again. Battery runtime depends on the measured assembled device and is
not universally guaranteed.

Pokeviewer is an adult-built, child-adjacent development-board project, not a
certified finished toy. No board, enclosure, battery, charger, or cable is
included. Read the safety guide and complete the polarity gate before
connecting a protected single-cell battery.

Publication is permitted only after the
[seven-day physical qualification](https://github.com/timbrinded/pokeviewer/issues/24)
and
[exact-artifact clean-host rehearsal](https://github.com/timbrinded/pokeviewer/issues/27)
are closed with reviewed evidence.

This is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

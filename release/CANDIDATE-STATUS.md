# Pokeviewer v1.1.0 candidate status

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board.

The display shows the weekday, a Pokémon Yellow sprite, English name, and
canonical type or types. The selection changes deterministically at 07:00
local time. There is no runtime internet, Wi-Fi, BLE, touch, account, location,
SD-card, or over-the-air update dependency.

This candidate adds a PWR-gated parent session, storage mode, EXT1 wake for the
RTC alarm and PWR button, and an approximate battery display. The battery
value is not a fuel gauge or a safety control.

Complete RTC power loss requires an adult to reconnect USB and set the local
clock again. Read `USER-GUIDE.md`, `SAFETY.md`, and `TROUBLESHOOTING.md` before
use.

This candidate is not physically qualified or approved for publication until
the v1.1.0 device qualification and exact-artifact clean-host rehearsal are
complete. The firmware implements the ESP-IDF-aligned RTC and PWR wake path.
Battery-only scheduled wake and PWR-gated parent setup passed on the target
board. Storage mode, simultaneous alarm and PWR wake, recovery injection,
the remaining repeated 07:00 transitions, and the seven-day run remain
unqualified.

Manual current measurement, battery-capacity measurement, and charger
certification are outside this release scope. Battery runtime is not
guaranteed.

It is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

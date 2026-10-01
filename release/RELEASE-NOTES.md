# Pokeviewer v1.2.0

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 development board.

The display shows the weekday, a Pokémon Yellow sprite, English name, and
canonical type or types. The selection changes deterministically at 07:00
local time. There is no runtime internet, Wi-Fi, BLE, touch, account, location,
SD-card, or over-the-air update dependency.

The release includes the parent session introduced in v1.1.0. Connect USB and
hold `PWR` for three seconds to change the time or enter storage mode. A short
`PWR` press has no visible effect. The `BOOT` button is for service and flashing
only.

The display now shows `Normal`, `Recharge`, or `Unavailable` battery state
instead of a percentage. `Normal` has no battery text or icon, `Recharge`
shows the existing lightning icon and `CHARGE!`, and `Unavailable` shows
`BAT ?`. `Recharge` enters below 3,750 mV and clears at or above 3,850 mV.
Only a validated RTC alarm wake commits the retained battery snapshot. The new
`pokeviewerctl get-battery` command reports that snapshot as state and bounded
cell millivolts. It does not report precise capacity and does not take a new
PWR-session sample.

Storage mode clears the RTC and turns off the board. The next start requires
time setup. Complete RTC power loss also requires an adult to connect USB and
set the local clock again.

The release contains one merged firmware image flashable at offset `0x0`, one
Linux x86-64 provisioning CLI, the compiled offline content pack, SHA-256
checksums, and adult setup, safety, and troubleshooting documentation. Flash
the board only while the battery is disconnected.

Pokeviewer is an adult-built, child-adjacent development-board project, not a
certified finished toy. No board, enclosure, battery, charger, or cable is
included. Battery runtime is not guaranteed. Read the safety guide before you
connect a protected single-cell battery.

This is an unofficial, non-commercial fan project and is not affiliated with,
endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc., or The
Pokémon Company International.

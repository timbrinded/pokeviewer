# Product contract

- Status: accepted
- Decision issue: [P01 / #2][issue-2]

This document is the authoritative product boundary. Change it in the same
pull request as the behaviour it governs.

## Supported device

- Waveshare ESP32-S3-ePaper-1.54-EN, V2 hardware revision.
- ESP32-S3-PICO-1-N8R8 with 8 MB flash and 8 MB PSRAM.
- Integrated non-touch, 1.54-inch, 200 × 200 black-and-white e-paper panel.
- Battery-powered operation through the board's supported 3.7 V lithium
  battery input.

The [V2 board contract](hardware/v2-board-contract.md) records pins and power
behavior. Vendor V1 and V2 examples are not interchangeable.

## Runtime contract

- Firmware is Rust and `no_std`.
- The released device is fully offline. Wi-Fi and Bluetooth are not initialized.
- Content and sprites are converted on a maintainer workstation and compiled
  into the release image.
- The onboard PCF85063 RTC owns local wall-clock time and the daily alarm.
- Full battery exhaustion may invalidate the RTC. An adult restores it over
  wired USB using the Linux x86-64 `pokeviewerctl` utility.
- An adult can also correct a valid RTC through a PWR-gated USB session.
- The device wakes for the 07:00 local display-day boundary, refreshes only
  when needed, and returns to deep sleep.
- E-paper retains the complete prior card until a successful refresh.
- A normal PWR tap does not change the display.
- BOOT enters service flashing when held at power-up with the battery
  disconnected. A one-second BOOT hold during operation restarts the firmware.
- The green LED shows adult USB-session and restart feedback only.

## Daily card

Every normal card contains four primary information groups:

1. weekday;
2. Pokémon Crystal front sprite, shaded with black-and-white dot patterns;
3. English Pokémon name; and
4. current canonical type or types.

The card also contains one non-interactive battery state: `Normal`, `Recharge`,
or `Unavailable`. `Normal` shows no battery text or icon. `Recharge` shows a
custom lightning icon and `CHARGE!`. `Unavailable` shows `BAT ?`. The card
does not show a percentage.

The content set is National Pokédex IDs 1 through 251, Generations I and II.
A fixed, versioned, non-repeating permutation selects one entry per display day
and repeats after 251 display days.

Before 07:00, the display day is the previous calendar date. This includes the
weekday: the entire previous card remains visible rather than mixing a new
weekday with yesterday's Pokémon.

## Explicit exclusions

V1 has no:

- runtime internet access, Wi-Fi, Bluetooth, accounts, telemetry, or cloud;
- touch support, child-facing button actions, menus, choices, scores, streaks,
  or games;
- audio, speech, animation, colour, or partial-refresh effects, or panel
  greyscale waveforms (sprite shading uses only black and white pixels);
- SD-card dependency or runtime content update;
- localization, descriptions, stats, moves, evolutions, or generations after
  Generation II;
- configurable wake time, timezone database, or automatic daylight-saving
  adjustment; or
- guaranteed battery runtime independent of the selected battery's measured
  capacity and condition.

Battery state comes from a bounded voltage sample. It is not a fuel gauge and
does not control shutdown, charging, or safety. The product makes no precise
capacity or runtime claim. The board has no dedicated USB VBUS-sense input,
so firmware cannot use a USB-powered reading to identify cell capacity.

## Distribution

Each GitHub release contains:

- one merged, ready-to-flash image for the supported V2 board;
- one Linux x86-64 `pokeviewerctl` binary;
- SHA-256 checksums and build/content version metadata;
- setup, operation, safety, and recovery documentation; and
- applicable licenses and third-party notices.

The [publishing guide](development/publishing.md) lists the exact files.

Original Pokeviewer code is MIT-licensed. Pokémon media is not. The
[third-party notice](../THIRD_PARTY_NOTICES.md) records the non-affiliation and
redistribution risk without claiming permission.

## Privacy

Nothing published from this repository may identify the child or household or
expose a device identifier. The [privacy rules](privacy-and-evidence.md) list
what to leave out.

## Sources

- [Waveshare ESP32-S3-ePaper-1.54 documentation][waveshare]
- [PokéAPI v2 documentation and fair-use policy][pokeapi]
- [PokéAPI Pokémon Crystal sprite tree][sprites]
- [The Pokémon Company International legal information][pokemon-legal]

[issue-2]: https://github.com/timbrinded/pokeviewer/issues/2
[pokeapi]: https://pokeapi.co/docs/v2
[pokemon-legal]: https://www.pokemon.com/us/legal/information
[sprites]: https://github.com/PokeAPI/sprites/tree/master/sprites/pokemon/versions/generation-ii/crystal
[waveshare]: https://docs.waveshare.com/ESP32-S3-ePaper-1.54

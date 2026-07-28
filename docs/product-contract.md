# V1 product contract

- Status: accepted
- Decision issue: [P01 / #2][issue-2]
- Last reviewed: 2026-07-27

This document is the authoritative product boundary for v1. Any change to a
locked decision requires a dedicated decision issue before implementation.

## Supported device

- Waveshare ESP32-S3-ePaper-1.54-EN, V2 hardware revision.
- ESP32-S3-PICO-1-N8R8 with 8 MB flash and 8 MB PSRAM.
- Integrated non-touch, 1.54-inch, 200 × 200 black-and-white e-paper panel.
- Battery-powered operation through the board's supported 3.7 V lithium
  battery input.

The exact V2 pin and power contract is maintained separately because vendor V1
and V2 examples are not interchangeable.

## Runtime contract

- Firmware is Rust and `no_std`.
- The released device is fully offline. Wi-Fi and Bluetooth are not initialized.
- Content and sprites are converted on a maintainer workstation and compiled
  into the release image.
- The onboard PCF85063 RTC owns local wall-clock time and the daily alarm.
- Full battery exhaustion may invalidate the RTC. An adult restores it over
  wired USB using the Linux x86-64 `pokeviewerctl` utility.
- The device wakes for the 07:00 local display-day boundary, refreshes only
  when needed, and returns to deep sleep.
- E-paper retains the complete prior card until a successful refresh.

## Daily card

Every normal card contains exactly four information groups:

1. weekday;
2. Pokémon Yellow front sprite;
3. English Pokémon name; and
4. current canonical type or types.

The content set is National Pokédex IDs 1 through 151. A fixed, versioned,
non-repeating permutation selects one entry per display day and repeats after
151 display days.

Before 07:00, the display day is the previous calendar date. This includes the
weekday: the entire previous card remains visible rather than mixing a new
weekday with yesterday's Pokémon.

## Explicit exclusions

V1 has no:

- runtime internet access, Wi-Fi, Bluetooth, accounts, telemetry, or cloud;
- touch support, child-facing buttons, menus, choices, scores, streaks, or
  games;
- audio, speech, animation, greyscale, colour, or partial-refresh effects;
- SD-card dependency or runtime content update;
- localization, descriptions, stats, moves, evolutions, or generations after
  Generation I;
- configurable wake time, timezone database, or automatic daylight-saving
  adjustment; or
- guaranteed battery runtime independent of the selected battery's measured
  capacity and condition.

## Distribution

V1 publishes one final `v1.0.0` GitHub release, not a prerelease. It contains:

- one merged, ready-to-flash image for the supported V2 board;
- one Linux x86-64 `pokeviewerctl` binary;
- SHA-256 checksums and build/content version metadata;
- setup, operation, safety, recovery, and verification documentation; and
- applicable licenses and third-party notices.

Original Pokeviewer code is MIT-licensed. Pokémon media is not. The
[third-party notice](../THIRD_PARTY_NOTICES.md) records the non-affiliation and
redistribution risk without claiming permission.

## Public evidence boundary

Screenshots, photographs, logs, CI artifacts, and release evidence must follow
the [privacy and evidence rules](privacy-and-evidence.md). In particular, they
must not expose a child's identity, home information, credentials, private host
paths, device MAC address, or USB serial identifier.

## Sources

- [Waveshare ESP32-S3-ePaper-1.54 documentation][waveshare]
- [PokéAPI v2 documentation and fair-use policy][pokeapi]
- [PokéAPI Pokémon Yellow sprite tree][sprites]
- [The Pokémon Company International legal information][pokemon-legal]

[issue-2]: https://github.com/timbrinded/pokeviewer/issues/2
[pokeapi]: https://pokeapi.co/docs/v2
[pokemon-legal]: https://www.pokemon.com/us/legal/information
[sprites]: https://github.com/PokeAPI/sprites/tree/master/sprites/pokemon/versions/generation-i/yellow
[waveshare]: https://docs.waveshare.com/ESP32-S3-ePaper-1.54

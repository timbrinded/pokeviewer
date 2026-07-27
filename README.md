# Pokeviewer

Pokeviewer is a battery-powered, fully offline Pokémon-of-the-day display for
the non-touch Waveshare ESP32-S3-ePaper-1.54-EN V2 board.

The project is at the start of its v1 implementation. The authoritative scope
is the [v1 product contract](docs/product-contract.md); work is tracked under
the [v1.0.0 delivery issue](https://github.com/timbrinded/pokeviewer/issues/1).
Firmware supports only the
[documented V2 board contract](docs/hardware/v2-board-contract.md).

## V1 experience

Once a day at 07:00 local time, the device:

1. wakes from deep sleep;
2. selects the next entry from a fixed 151-day Generation I rotation;
3. displays the weekday, Pokémon Yellow sprite, English name, and current
   canonical type or types; and
4. returns to deep sleep while the e-paper retains the card.

There is no child-facing interaction and no runtime network access.

## Important notices

Pokeviewer is an unofficial, non-commercial fan project. It is not affiliated
with, endorsed by, or sponsored by Nintendo, Creatures Inc., GAME FREAK inc.,
or The Pokémon Company International.

The project's MIT license will cover original source code only. Pokémon names,
characters, artwork, sprites, and related media are third-party property and
are excluded from that license. See [Third-party notices](THIRD_PARTY_NOTICES.md)
before redistributing a build or asset pack.

Public project evidence must follow the
[privacy and evidence rules](docs/privacy-and-evidence.md).

## Decisions

Significant decisions are recorded in the
[architecture decision log](docs/decisions/README.md).

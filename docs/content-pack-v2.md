# Content-pack and daily-schedule contract v2

- Status: current
- Contract issues: [D09 / #10][issue-10], [#40][issue-40]
- Binary format version: 2
- Content revision: 3
- Schedule version: 2

This contract defines the only Pokémon data consumed by the firmware and the
mapping from local civil time to one daily record. It implements the content
and display-day rules of the [product contract](product-contract.md). The
closed decision records
[0001](decisions/0001-compile-an-offline-content-pack.md) and
[0002](decisions/0002-use-a-passive-0700-display-day.md) give the original
rationale; this contract replaces their Generation I, 151-record scope.

## Content scope

The pack contains exactly one record for each National Pokédex ID from 1
through 251, ordered by ID. Each record contains only:

- the National Pokédex ID;
- the approved English display name;
- one current canonical primary type and, when applicable, one distinct
  current canonical secondary type; and
- one 56 × 56 sprite of two-bit shades derived from the Pokémon Crystal front
  sprite. Unown (201) uses its default form, Unown A.

Names are UTF-8, 1–16 bytes long, with no control characters, including NUL;
the firmware parser enforces these rules. The converter also normalizes names
to NFC. Besides letters, committed names use a space and a period
(Mr. Mime), the curly apostrophe U+2019 (Farfetch’d), a hyphen (Ho-Oh), the
digit 2 (Porygon2), and the female and male signs (Nidoran♀, Nidoran♂). The
renderer's fixed glyph set must cover every committed name; an exhaustive
host test renders all 251.

Type values are stable pack codes, not upstream strings:

| Code | Type | Code | Type | Code | Type |
| ---: | --- | ---: | --- | ---: | --- |
| 0 | Normal | 6 | Fighting | 12 | Rock |
| 1 | Fire | 7 | Poison | 13 | Ghost |
| 2 | Water | 8 | Ground | 14 | Dragon |
| 3 | Electric | 9 | Flying | 15 | Dark |
| 4 | Grass | 10 | Psychic | 16 | Steel |
| 5 | Ice | 11 | Bug | 17 | Fairy |

`0xff` means that no secondary type exists. Unknown codes, duplicate primary
and secondary types, and missing or extra IDs invalidate the complete pack.

### Sprite conversion

The source is the unmodified Pokémon Crystal front PNG in the accepted
maintainer cache, `content/cache-v2`. Conversion:

1. requires a non-empty source no larger than 56 × 56;
2. centers the native source pixels on a 56 × 56 canvas without scaling,
   cropping, or interpolation; an odd spare pixel is placed on the right or
   bottom;
3. treats source pixels with alpha below 128 as transparent and excludes them
   from the palette;
4. requires every other source pixel to be fully opaque;
5. collects their distinct RGB values and requires exactly four source colours,
   with white (`255, 255, 255`) as the lightest;
6. orders those four colours by integer luminance,
   `(299 * red + 587 * green + 114 * blue + 500) / 1000`; among equal
   luminances, the colour with the lower red, then green, then blue value is
   darker;
7. writes the colours from lightest to darkest as shades `0` (white), `1`,
   `2`, and `3` (black), and writes transparent pixels and canvas padding as
   `0`; and
8. applies no dithering. The renderer owns how shades appear on the panel; see
   [rendering](development/rendering.md#sprite-shading).

For eight sprites (21, 22, 83, 106, 123, 124, 137, and 233), the
pret/pokecrystal build reverses the palette (`--reverse`), so luminance order
differs from the original Game Boy shade order. The converter applies the
luminance rule to every sprite and keeps no exception list.

## Binary format

All integers are unsigned little-endian. The file has no padding, timestamps,
host paths, JSON, or platform-dependent values:

```text
32-byte header
251 fixed-size records
concatenated UTF-8 name bytes
251-byte schedule permutation
251 fixed-size sprite bitmaps
```

### Header

| Offset | Size | Field | Required v2 value |
| ---: | ---: | --- | --- |
| 0 | 4 | magic | ASCII `PKVW` |
| 4 | 2 | format version | `2` |
| 6 | 2 | header length | `32` |
| 8 | 4 | content revision | `3` |
| 12 | 2 | schedule version | `2` |
| 14 | 2 | record count | `251` |
| 16 | 2 | permutation count | `251` |
| 18 | 1 | sprite width | `56` |
| 19 | 1 | sprite height | `56` |
| 20 | 1 | record size | `6` |
| 21 | 1 | flags | `0`; other bits are invalid |
| 22 | 2 | names length | actual byte length, at most `4016` |
| 24 | 4 | payload length | all bytes following the header |
| 28 | 4 | payload CRC | CRC-32/ISO-HDLC of the complete payload |

CRC-32/ISO-HDLC uses polynomial `0x04c11db7`, reflected input and output,
initial value `0xffffffff`, and final XOR `0xffffffff`.

### Record

Each six-byte record contains, in order:

| Size | Field | Rule |
| ---: | --- | --- |
| 1 | Pokédex ID | record ordinal plus one |
| 1 | primary type | `0`–`17` |
| 1 | secondary type | `0`–`17` or `0xff` |
| 1 | name length | `1`–`16` |
| 2 | name offset | offset within the names section |

Name slices are contiguous in record order, non-overlapping, and exactly cover
the names section.

### Sprite bitmap

Each sprite is 784 bytes of two-bit shades. Pixels serialize by row from top
to bottom and within each row from left to right; the most-significant bit
pair of a byte is the leftmost pixel, and fourteen bytes encode each 56-pixel
row. Every two-bit value is a valid shade. Sprite ordinal and record ordinal
are identical, so no sprite offset is stored.

### Deterministic serialization

The generator must:

- read only the accepted cache and its provenance manifest;
- process records in ascending Pokédex ID order;
- use the type codes and sprite conversion above;
- concatenate names in record order;
- emit the exact schedule-v2 permutation below;
- write the flags field as zero;
- compute lengths and CRC only after the payload is complete; and
- produce byte-identical output from the same cache on repeated runs.

Normal CI validates the committed cache and pack without accessing PokéAPI.
Cache refreshes (`cargo xtask content-fetch`) are explicit maintainer actions.

## Schedule v2

The epoch display date is 2026-01-01 in the RTC's already-provisioned local
civil time. For a local datetime:

```text
display_date = local_date                 when local_time >= 07:00:00
display_date = local_date - one day       when local_time <  07:00:00
cycle_index = days(display_date - 2026-01-01) rem_euclid 251
dex_id = ((94 * cycle_index) mod 251) + 1
```

Date arithmetic uses the proleptic Gregorian calendar. Negative differences
use Euclidean modulo, so dates before the epoch are defined rather than
underflowing.

This mapping is the repository-owned schedule-v2 permutation. Because 251 is
prime, every ID appears exactly once before the cycle repeats. Within any seven
consecutive display days, every two IDs are at least 31 apart. The pack stores
all 251 resulting IDs in cycle-index order; the parser rejects a list that
differs from the formula. Firmware selects from those stored bytes and does
not run a PRNG.

A restart before 07:00 must never combine the new calendar weekday with the
prior Pokémon. If the restart path renders a card, both weekday and Pokémon
come from `display_date`; otherwise the prior card stays on the panel.

### Worked examples

| Local datetime | Display date and weekday | Index | Pokédex ID | Reason |
| --- | --- | ---: | ---: | --- |
| 2025-12-31 12:00 | 2025-12-31, Wednesday | 250 | 158 | one day before epoch wraps backward |
| 2026-01-01 06:59:59 | 2025-12-31, Wednesday | 250 | 158 | prior card remains before 07:00 |
| 2026-01-01 07:00:00 | 2026-01-01, Thursday | 0 | 1 | epoch boundary |
| 2026-01-02 12:00 | 2026-01-02, Friday | 1 | 95 | ordinary next display day |
| 2026-09-08 23:59 | 2026-09-08, Tuesday | 250 | 158 | final cycle entry |
| 2026-09-09 06:59:59 | 2026-09-08, Tuesday | 250 | 158 | final entry retained after midnight |
| 2026-09-09 07:00:00 | 2026-09-09, Wednesday | 0 | 1 | entry 251 wraps to entry 1 |
| 2024-03-01 06:59:59 | 2024-02-29, Thursday | 81 | 85 | leap-day rollback |
| 2000-01-01 00:00:00 | 1999-12-31, Friday | 40 | 247 | earliest supported RTC reading |
| 2099-12-31 23:59:59 | 2099-12-31, Thursday | 170 | 168 | latest supported RTC reading |

## Compatibility and failure policy

Firmware embeds the pack with `include_bytes!` and parses bounded byte
slices without allocation or runtime JSON. When the RTC holds a valid time,
firmware validates the pack before it renders the daily card and powers the
panel. It checks:

- magic, exact format version, header and record sizes, flags, and all lengths;
- exact supported content revision and schedule version;
- CRC;
- record order, names, type codes, and the schedule-v2 permutation;
- a sprite section of exactly 251 × 784 bytes; and
- that the total input is consumed with no trailing bytes.

With an invalid RTC, firmware shows the setup screen without reading the pack.

Format changes require a new format version. Content or schedule changes require
their own reviewed revision and a firmware release. Firmware rejects
unsupported versions rather than attempting forward compatibility, so it does
not read format-1 packs.

On any validation failure, firmware selects no record and shows the `PACK` /
`REFLASH` recovery screen described in the
[failure table](hardware/wake-sleep-state-machine.md#failure-path). A renderer
rejection of the selected record, or a stored ID that differs from the
schedule, shows the same screen. An adult recovers the device by flashing a
verified release.

### Changes from v1

Format 1 (content revisions 1 and 2, schedule v1) held the 151 Generation I
Pokémon with one-bit Pokémon Yellow sprites and selected
`(73 * cycle_index) mod 151`. Schedule v2 keeps the epoch and the 07:00
rollover. A device updated from a v1 image selects from the v2 permutation from
its next display refresh. All 251 Pokémon, including Generation I, now use
Crystal sprites, so every card shares one art style.

## Size budget

| Section | Maximum bytes |
| --- | ---: |
| header | 32 |
| 251 records | 1,506 |
| names | 4,016 |
| permutation | 251 |
| metadata subtotal | 5,805 |
| 251 sprites | 196,784 |
| complete pack | 202,589 |

The hard limit is 262,144 bytes (256 KiB). A pack at the maximum size above
still leaves 59,555 bytes of headroom; the committed pack is 200,410 bytes.
Firmware, fonts, stack, heap, and framebuffers have separate budgets, so the
256 KiB pack limit is not a claim about total flash or RAM use. Firmware keeps
the pack in flash and decodes only one fixed record and sprite at a time.

## Provenance and redistribution

The cache manifest records one retrieval time for the fetch, the pinned
PokeAPI/sprites revision, and each cached file's source URL, repository path,
and SHA-256 digest. The generator rejects a cache whose schema version,
sprite revision, URLs, paths, or digests differ from the expected values. The
generated pack manifest records the cache-manifest digest, converter version,
format, content, and schedule versions, sprite revision, pack length, pack
SHA-256 digest, and contact-sheet digest.

PokéAPI and its sprite repository provide technical provenance, not a Pokémon
media license. This non-commercial fan project accepts the redistribution risk
recorded in the [product contract](product-contract.md) and
[third-party notice](../THIRD_PARTY_NOTICES.md). Original code remains
MIT-licensed; Pokémon names, characters, artwork, sprites, and related media
are excluded from that license.

[issue-10]: https://github.com/timbrinded/pokeviewer/issues/10
[issue-40]: https://github.com/timbrinded/pokeviewer/issues/40

# Content-pack and daily-schedule contract v2

- Status: proposed with [ADR 0012](decisions/0012-show-generations-one-and-two-with-shaded-crystal-sprites.md)
- Contract issues: [D09 / #10][issue-10], [#40][issue-40]
- Binary format version: 2
- Content revision: 3
- Schedule version: 2

This contract defines the only Pokémon data consumed by the firmware and the
mapping from local civil time to one daily record. It refines the accepted
[offline-pack decision](decisions/0001-compile-an-offline-content-pack.md) and
[07:00 display-day decision](decisions/0002-use-a-passive-0700-display-day.md).
Format 1, content revision 2, and schedule v1 held the 151 Generation I
Pokémon with one-bit Pokémon Yellow sprites.

## Content scope

The pack contains exactly one record for each National Pokédex ID from 1
through 251, ordered by ID. Each record contains only:

- the National Pokédex ID;
- the approved English display name;
- one current canonical primary type and, when applicable, one distinct
  current canonical secondary type; and
- one 56 × 56 sprite of two-bit shades derived from the Pokémon Crystal front
  sprite. Unown (201) uses its default form, Unown A.

Names are NFC-normalized UTF-8, 1–16 bytes long, with no control or NUL
characters. The renderer's reviewed glyph set must cover every committed
name, including the female and male symbols used by Nidoran, the hyphen in
Ho-Oh, and the digit in Porygon2.

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

The source is the unmodified Pokémon Crystal front PNG from the explicit
maintainer cache. Conversion:

1. requires a non-empty source no larger than 56 × 56;
2. centers the native source pixels on a transparent 56 × 56 output canvas
   without scaling, cropping, or interpolation; an odd spare pixel is placed
   on the right or bottom;
3. treats alpha values below 128 as white and excludes them from the source
   palette;
4. requires every other source pixel to be fully opaque;
5. collects their distinct RGB values and requires exactly four source colours,
   with white (`255, 255, 255`) as the lightest;
6. orders those four colours by integer luminance,
   `(299 * red + 587 * green + 114 * blue + 500) / 1000`;
   ties are resolved by red, then green, then blue;
7. writes the colours from lightest to darkest as shades `0` (white), `1`,
   `2`, and `3` (black), and writes transparent and out-of-source canvas
   pixels as `0`; and
8. applies no dithering. The renderer owns how shades appear on the panel; see
   [rendering](development/rendering.md#sprite-shading).

Luminance order differs from the original Game Boy shade order for eight
sprites (21, 22, 83, 106, 123, 124, 137, and 233). The converter keeps the
luminance rule for them;
[ADR 0012](decisions/0012-show-generations-one-and-two-with-shaded-crystal-sprites.md)
records why.

Output-canvas pixels serialize by row from top to bottom and within each row
from left to right. Each pixel is two bits; the most-significant pair of a
byte is the leftmost pixel. Fourteen bytes encode each 56-pixel row, so every
sprite is exactly 784 bytes.

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
| 8 | 4 | content revision | `3`; revisions `1` and `2` held one-bit Yellow sprites |
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
the names section. Sprite ordinal and record ordinal are identical, so no
sprite offset is stored.

### Deterministic serialization

The generator must:

- read only an explicit cache and its provenance manifest;
- process records in ascending Pokédex ID order;
- use the type codes and sprite conversion above;
- concatenate names in record order;
- emit the exact schedule-v2 permutation below;
- write every reserved or flags field as zero;
- compute lengths and CRC only after the payload is complete; and
- produce byte-identical output from the same cache on repeated runs.

Normal CI validates the committed cache and pack without accessing PokéAPI.
Refreshes are explicit maintainer actions.

## Schedule v2

The epoch display date is 2026-01-01 in the RTC's already-provisioned local
civil time. For a local datetime:

```text
display_date = local_date                 when local_time >= 07:00:00
display_date = local_date - one day       when local_time <  07:00:00
cycle_index = days(display_date - 2026-01-01) rem_euclid 251
dex_id = ((94 * cycle_index) mod 251) + 1
```

This mapping is the repository-owned schedule-v2 permutation. Because 251 is
prime, every ID appears exactly once before the cycle repeats. Within any seven
consecutive display days, every two IDs are at least 31 apart, the largest
spacing any multiplier allows. The pack stores all 251 resulting IDs in
cycle-index order; the parser rejects a list that differs from the formula.
Firmware selects from those stored bytes and does not run a PRNG.

Schedule v1 used `(73 * cycle_index) mod 151`. A device that updates from v1
shows a different Pokémon from the next refresh; the epoch and rollover are
unchanged.

Date arithmetic uses the proleptic Gregorian calendar. Negative differences
use Euclidean modulo, so dates before the epoch are defined rather than
underflowing.

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

A restart before 07:00 must never combine the new calendar weekday with the
prior Pokémon. If recovery requires rendering, both weekday and Pokémon come
from `display_date`; otherwise the retained prior card remains untouched.

## Compatibility and failure policy

Firmware embeds the pack with `include_bytes!` and parses bounded byte
slices without allocation or runtime JSON. Before powering the panel, it
validates:

- magic, exact format version, header and record sizes, flags, and all lengths;
- exact supported content revision and schedule version;
- CRC;
- record, name, type, permutation, and sprite invariants; and
- that the total input is consumed with no trailing bytes.

Format changes require a new format version. Content or schedule changes require
their own reviewed revision and a firmware release. Firmware rejects
unsupported versions rather than attempting forward compatibility.

On any validation failure, firmware selects no record and shows the `PACK` /
`REFLASH` recovery screen described in the
[failure table](hardware/wake-sleep-state-machine.md#failure-path). An adult
recovers the device by flashing a verified release.

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

The committed pack is 200,410 bytes. The hard limit is 262,144 bytes, leaving
at least 59,555 bytes of pack-level headroom. Firmware, fonts, stack, heap, and framebuffers have separate budgets;
the 64 KiB pack limit is not a claim about total flash or RAM use. Firmware
keeps the pack in flash and decodes only one fixed record and sprite at a time.

## Provenance and redistribution

Every cached response and source PNG must have its source URL, retrieval time,
and SHA-256 digest recorded in the cache manifest. The generated pack manifest
records the cache-manifest digest, converter version, format/content/schedule
versions, pack length, and pack SHA-256 digest.

PokéAPI and its sprite repository provide technical provenance, not a Pokémon
media license. This non-commercial fan project accepts the redistribution risk
recorded in the [product contract](product-contract.md) and
[third-party notice](../THIRD_PARTY_NOTICES.md). Original code remains
MIT-licensed; Pokémon names, characters, artwork, sprites, and related media
are excluded from that license.

[issue-10]: https://github.com/timbrinded/pokeviewer/issues/10
[issue-40]: https://github.com/timbrinded/pokeviewer/issues/40

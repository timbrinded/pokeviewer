# Content-pack and daily-schedule contract v1

- Status: accepted
- Contract issue: [D09 / #10][issue-10]
- Binary format version: 1
- Content revision: 1
- Schedule version: 1
- Last reviewed: 2026-07-27

This contract defines the only Pokémon data consumed by v1 firmware and the
mapping from local civil time to one daily record. It refines the accepted
[offline-pack decision](decisions/0001-compile-an-offline-content-pack.md) and
[07:00 display-day decision](decisions/0002-use-a-passive-0700-display-day.md).

## Content scope

The pack contains exactly one record for each National Pokédex ID from 1
through 151, ordered by ID. Each record contains only:

- the National Pokédex ID;
- the approved English display name;
- one current canonical primary type and, when applicable, one distinct
  current canonical secondary type; and
- one 56 × 56 one-bit sprite derived from the Pokémon Yellow front sprite.

Names are NFC-normalized UTF-8, 1–16 bytes long, with no control or NUL
characters. The v1 renderer's reviewed glyph set must cover every committed
name, including the female and male symbols used by Nidoran.

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

The source is the unmodified Pokémon Yellow front PNG from the explicit
maintainer cache. Conversion:

1. requires a non-empty source no larger than 56 × 56;
2. centers the native source pixels on a transparent 56 × 56 output canvas
   without scaling, cropping, or interpolation; an odd spare pixel is placed
   on the right or bottom;
3. treats alpha values below 128 as white;
4. calculates integer luminance for other pixels as
   `(299 * red + 587 * green + 114 * blue + 500) / 1000`;
5. writes black when luminance is below 128 and white otherwise; and
6. applies no dithering.

Output-canvas pixels serialize by row from top to bottom and within each row
from left to right. The most-significant bit is the leftmost pixel, `1` is
black, and `0` is white. Seven bytes encode each 56-pixel row, so every sprite
is exactly 392 bytes.

## Binary format

All integers are unsigned little-endian. The file has no padding, timestamps,
host paths, JSON, or platform-dependent values:

```text
32-byte header
151 fixed-size records
concatenated UTF-8 name bytes
151-byte schedule permutation
151 fixed-size sprite bitmaps
```

### Header

| Offset | Size | Field | Required v1 value |
| ---: | ---: | --- | --- |
| 0 | 4 | magic | ASCII `PKVW` |
| 4 | 2 | format version | `1` |
| 6 | 2 | header length | `32` |
| 8 | 4 | content revision | `1` for the first accepted pack |
| 12 | 2 | schedule version | `1` |
| 14 | 2 | record count | `151` |
| 16 | 2 | permutation count | `151` |
| 18 | 1 | sprite width | `56` |
| 19 | 1 | sprite height | `56` |
| 20 | 1 | record size | `6` |
| 21 | 1 | flags | `0`; other bits are invalid |
| 22 | 2 | names length | actual byte length, at most `2416` |
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
- emit the exact schedule-v1 permutation below;
- write every reserved or flags field as zero;
- compute lengths and CRC only after the payload is complete; and
- produce byte-identical output from the same cache on repeated runs.

Normal CI validates the committed cache and pack without accessing PokéAPI.
Refreshes are explicit maintainer actions.

## Schedule v1

The epoch display date is 2026-01-01 in the RTC's already-provisioned local
civil time. For a local datetime:

```text
display_date = local_date                 when local_time >= 07:00:00
display_date = local_date - one day       when local_time <  07:00:00
cycle_index = days(display_date - 2026-01-01) rem_euclid 151
dex_id = ((73 * cycle_index) mod 151) + 1
```

This affine mapping is the repository-owned schedule-v1 permutation. Because
73 and 151 are coprime, every ID appears exactly once before the cycle repeats.
The pack stores all 151 resulting IDs in cycle-index order; the generator must
reject a list that differs from the formula, contains a duplicate, or omits an
ID. Firmware selects from those stored bytes and does not run a PRNG.

Date arithmetic uses the proleptic Gregorian calendar. Negative differences
use Euclidean modulo, so dates before the epoch are defined rather than
underflowing.

### Worked examples

| Local datetime | Display date and weekday | Index | Pokédex ID | Reason |
| --- | --- | ---: | ---: | --- |
| 2025-12-31 12:00 | 2025-12-31, Wednesday | 150 | 79 | one day before epoch wraps backward |
| 2026-01-01 06:59:59 | 2025-12-31, Wednesday | 150 | 79 | prior card remains before 07:00 |
| 2026-01-01 07:00:00 | 2026-01-01, Thursday | 0 | 1 | epoch boundary |
| 2026-01-02 12:00 | 2026-01-02, Friday | 1 | 74 | ordinary next display day |
| 2026-05-31 23:59 | 2026-05-31, Sunday | 150 | 79 | final cycle entry |
| 2026-06-01 06:59:59 | 2026-05-31, Sunday | 150 | 79 | final entry retained after midnight |
| 2026-06-01 07:00:00 | 2026-06-01, Monday | 0 | 1 | entry 151 wraps to entry 1 |

A restart before 07:00 must never combine the new calendar weekday with the
prior Pokémon. If recovery requires rendering, both weekday and Pokémon come
from `display_date`; otherwise the retained prior card remains untouched.

## Compatibility and failure policy

V1 firmware embeds the pack with `include_bytes!` and parses bounded byte
slices without allocation or runtime JSON. Before powering the panel, it
validates:

- magic, exact format version, header and record sizes, flags, and all lengths;
- exact supported content revision and schedule version;
- CRC;
- record, name, type, permutation, and sprite invariants; and
- that the total input is consumed with no trailing bytes.

Format changes require a new format version. Content or schedule changes require
their own reviewed revision and a firmware release. V1 firmware rejects
unsupported versions rather than attempting forward compatibility.

On any validation failure, firmware records a bounded sanitized error code,
does not select a record, and does not power or overwrite the e-paper. The last
valid retained card therefore remains visible; an adult recovers the device by
installing a compatible release artifact whose published SHA-256 checksum
matches.

## Size budget

| Section | Maximum bytes |
| --- | ---: |
| header | 32 |
| 151 records | 906 |
| names | 2,416 |
| permutation | 151 |
| metadata subtotal | 3,505 |
| 151 sprites | 59,192 |
| complete pack | 62,697 |

The hard v1 limit is 65,536 bytes, leaving at least 2,839 bytes of pack-level
headroom. Firmware, fonts, stack, heap, and framebuffers have separate budgets;
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

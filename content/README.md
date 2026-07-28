# Generation I content pack v1

This directory contains the reviewed inputs and generated outputs for the
firmware's complete offline National Pokédex 1–151 pack.

## Contents

- `cache-v1/manifest.json` maps every ID to exact Pokémon/species response and
  pinned sprite URLs, repository-relative files, and SHA-256 digests.
- `cache-v1/pokemon/` and `cache-v1/species/` contain the explicit PokéAPI
  responses used by the converter.
- `cache-v1/sprites/` contains Pokémon Yellow front sprites from
  PokeAPI/sprites commit
  `8dfa3d97e953caaafaafd4963eff7621811af08e`.
- `generated/pokeviewer-v1.pack` is the allocation-free firmware pack.
- `generated/pokeviewer-v1.json` is the machine-readable provenance and
  validation report.
- `generated/sprites-contact-sheet.png` shows converted sprites in row-major
  National Pokédex order. The final nine cells are intentionally empty.

## Accepted evidence

| Property | Value |
| --- | --- |
| records and unique IDs | 151 |
| ID range | 1–151 |
| valid converted sprites | 151 |
| encoded bytes per sprite | 392 |
| sprite bytes | 59,192 |
| metadata bytes | 2,198 |
| complete pack bytes | 61,390 |
| firmware decode heap | 0 bytes |
| display framebuffer | 5,000 bytes |
| pack SHA-256 | `339c42d721d7b0d3eec53c4ef538f4014b825a692d9da89ed8f051693cb88cf3` |
| contact-sheet SHA-256 | `9807c13d560c2bcc89d5dbf169529a1f03396f198fd2adc1d43907849b91a3dc` |

Two conversions from the committed cache produced the same pack hash and
passed a byte-for-byte `cmp`. `pokeviewer-core` validates the complete pack and
borrows all records, names, schedule entries, and fixed sprite slices directly
from the pack bytes without heap allocation.

## Regeneration

Regeneration must use the repository tool:

```sh
cargo xtask content-build
```

Do not hand-edit generated files. See the
[content tooling guide](../docs/development/content-tooling.md) and
[content-pack contract](../docs/content-pack-v1.md).

## Rights

Pokeviewer does not claim ownership of Pokémon names, characters, types, or
sprites. These cached responses and derived images are third-party material and
are not covered by the repository's MIT license. See
[Third-party notices](../THIRD_PARTY_NOTICES.md) before redistributing them.

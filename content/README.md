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
| pack SHA-256 | `dd19d101fd5801d77e853f1c6cf682c5c46935a949afd386cb290e59142c681d` |
| contact-sheet SHA-256 | `655ca204904a80926a6a481ca1a095bae7127b7c34b17391e866ac943bec962c` |

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

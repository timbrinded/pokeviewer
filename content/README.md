# Generation I content pack v1

This directory holds the inputs and outputs for the firmware's offline
National Pokédex 1–151 pack.

- `cache-v1/manifest.json` maps every ID to its PokéAPI Pokémon and species
  responses and its sprite URL, with the repository file and SHA-256 digest of
  each.
- `cache-v1/pokemon/`, `cache-v1/species/`, and `cache-v1/sprites/` hold those
  responses and the Pokémon Yellow front sprites from PokeAPI/sprites commit
  `8dfa3d97e953caaafaafd4963eff7621811af08e`.
- `generated/pokeviewer-v1.pack` is the pack compiled into the firmware.
- `generated/pokeviewer-v1.json` is the provenance manifest, including the
  pack and contact-sheet hashes.
- `generated/sprites-contact-sheet.png` shows the converted sprites in
  Pokédex order. The last nine cells are empty.

Regenerate `generated/` only with `cargo xtask content-build`, never by hand.
CI fails if the committed files differ from a rebuild. See the
[content tooling guide](../docs/development/content-tooling.md) and the
[content-pack contract](../docs/content-pack-v1.md).

## Rights

Pokeviewer does not own Pokémon names, characters, types, or sprites. The
cached responses and derived images are third-party material and are not
covered by the repository's MIT license. Read the
[third-party notices](../THIRD_PARTY_NOTICES.md) before you redistribute them.

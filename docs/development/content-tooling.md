# Offline content tooling

Content has two steps: an explicit network fetch into a cache, and an offline
build from that cache. Builds, tests, and CI never run the fetch. The
[content-pack contract](../content-pack-v1.md) defines the format, conversion,
schedule, and size budget.

## Fetch a candidate cache

The fetch needs `curl` with HTTPS. The destination must not exist, so fetch
into a new directory and compare it with the accepted cache:

```sh
cargo xtask content-fetch content/cache-candidate
```

It fetches IDs 1–151 and records each Pokémon, species, and sprite URL, the
repository-relative file, and a SHA-256 digest, plus one retrieval time.
Sprites come from PokeAPI/sprites commit
`8dfa3d97e953caaafaafd4963eff7621811af08e`. The command rejects missing
fields, a missing English name, invalid types, and unusable sprites, and names
the Pokémon ID and rule that failed.

Review the manifest and source differences before you replace
`content/cache-v1`.

## Build the pack

```sh
cargo xtask content-build
```

The build reads only `content/cache-v1` and writes three files to
`content/generated`:

- `pokeviewer-v1.pack`, the firmware pack;
- `pokeviewer-v1.json`, the provenance manifest that maps each entry to its
  source hashes and records the pack and contact-sheet hashes; and
- `sprites-contact-sheet.png`, every converted sprite in Pokédex order.

It validates every ID, URL, path, digest, name, type, sprite size, and palette,
the schedule, and the 64 KiB limit. It builds the pack twice in memory and
fails if the bytes differ. CI runs this command and fails if any of the three
files changes.

To build from another cache, pass the cache, pack, and manifest paths. The
contact sheet is written next to the pack, so keep candidate output out of
`content/generated`:

```sh
cargo xtask content-build \
  content/cache-candidate \
  target/content-candidate/pokeviewer-v1.pack \
  target/content-candidate/pokeviewer-v1.json
```

Do not edit generated files by hand.

## Tests

```sh
cargo test -p xtask --locked
```

The tests use generated PNG data and PokéAPI fixtures. They cover type
ordering, palette splitting, rejection of unexpected palettes, malformed IDs,
invalid sprite sizes, and repeatable serialization.

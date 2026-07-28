# Offline content tooling

Content maintenance is split into an explicit network step and a deterministic
offline step. Normal Cargo builds, firmware builds, tests, and CI do not invoke
the network step.

## Requirements

- the repository's pinned host Rust toolchain;
- `curl` with HTTPS support, only for an explicit cache refresh; and
- enough disk space for 151 Pokémon responses, species responses, and sprites.

The accepted sprite source is pinned to PokeAPI/sprites commit
`8dfa3d97e953caaafaafd4963eff7621811af08e`. Pokémon and species endpoint
responses are locked by exact URLs and SHA-256 digests in the cache manifest.

## Refresh a candidate cache

Run:

```sh
cargo xtask content-fetch
```

This fetches only IDs 1–151 into `content/cache-v1`. For each ID it records:

- the exact Pokémon, species, and revision-pinned sprite URL;
- repository-relative cache paths;
- a SHA-256 digest for every response; and
- one cache-level retrieval time and sprite revision.

The command validates IDs, required response fields, English name, current
types, Yellow sprite URL, PNG dimensions, and conversion suitability before
accepting the cache. A failure includes the Pokémon ID and violated rule.

The destination must not already exist. To review upstream changes without
overwriting an accepted cache, provide a candidate path:

```sh
cargo xtask content-fetch content/cache-candidate
```

Diff the candidate manifest and source hashes deliberately. Do not refresh
content in CI or as a side effect of a firmware build.

## Build the pack offline

Once a reviewed cache exists:

```sh
cargo xtask content-build
```

The command reads only local files and writes:

- `content/generated/pokeviewer-v1.pack`; and
- `content/generated/pokeviewer-v1.json`, the deterministic provenance
  manifest mapping every packed entry to its three source hashes.

It validates all 151 IDs in order, exact URLs and paths, every source digest,
name/type rules, bounded native PNG dimensions, deterministic centering on the
56 × 56 output canvas, the exact four-colour source palette, deterministic
darkest-two palette splitting, schedule v1, section bounds, and the 64 KiB pack
limit. It builds twice in memory and fails if the bytes differ.

Optional positional arguments select a reviewed cache and separate output
files:

```sh
cargo xtask content-build \
  content/cache-candidate \
  content/generated/candidate.pack \
  content/generated/candidate.json
```

No converter output contains host paths, credentials, device identifiers, or
child-related data. The generated manifest stores only the pack file name.

## Reproducibility evidence

Build twice from the same cache into different files:

```sh
cargo xtask content-build \
  content/cache-v1 target/content-proof/first.pack \
  target/content-proof/first.json
cargo xtask content-build \
  content/cache-v1 target/content-proof/second.pack \
  target/content-proof/second.json
sha256sum \
  target/content-proof/first.pack \
  target/content-proof/second.pack
cmp \
  target/content-proof/first.pack \
  target/content-proof/second.pack
```

The two pack hashes and bytes must match. Manifest bytes differ only if the
chosen pack file names differ, so pack bytes are the release reproducibility
boundary.

## Tests

The host suite uses generated 56 × 56 PNG data and representative PokeAPI
fixtures. It covers:

- out-of-order type normalization;
- exact four-colour palette-split conversion;
- rejection of unexpected source palettes;
- malformed response IDs;
- invalid sprite dimensions; and
- byte-identical repeated pack serialization.

Run:

```sh
cargo test -p xtask --locked
```

The [content-pack contract](../content-pack-v1.md) is authoritative for the
wire format, conversion arithmetic, schedule, size budget, compatibility, and
failure policy.

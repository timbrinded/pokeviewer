# Visual goldens

Raw 5,000-byte `.bin` files are the authoritative panel-native baselines.
Their adjacent one-bit PNGs are deterministic review projections. The manifest
ties each case to its Pokédex ID, weekday, English name, file paths, CRC-32, and
SHA-256 hashes.

The seven cases cover every weekday plus the widest name, punctuation and
symbol glyphs, single and dual types, small sprites, and 56 × 56 source-sprite
boundaries.

Check the committed baselines without network or hardware:

```console
cargo xtask golden-check target/visual-diff
```

An intentional visual change requires the explicit maintainer command:

```console
cargo xtask golden-update
cargo xtask golden-check target/visual-diff
```

Review both the raw/hash manifest change and every PNG before committing it.
Normal builds and tests never update these files.

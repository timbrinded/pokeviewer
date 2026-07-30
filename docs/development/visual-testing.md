# Visual golden testing

Visual tests compare exact panel-native bytes, not lossy screenshots or
subjective image scores. Nine reviewed cases cover all weekdays, distinct
layout risks, the low-battery warning, and unavailable battery data.

## Normal check

```console
cargo xtask golden-check target/visual-diff
```

The command validates the committed manifest and file hashes, renders each case
from the committed offline pack, and compares all 5,000 bytes. It exits
non-zero if any pixel differs. Normal CI uses the same command and uploads the
failure directory as `visual-golden-diff`.

A failure directory contains:

- `*-expected.png`, generated from the committed raw baseline;
- `*-actual.png`, generated from the current renderer;
- `*-diff.png`, white where pixels match and black where they differ; and
- `*-report.txt`, with changed coordinates, count, and before/after SHA-256.

## Intentional update

Only use this after the design change has been reviewed:

```console
cargo xtask golden-update
cargo xtask golden-check target/visual-diff
```

The update command replaces the bounded `tests/goldens/cards` directory and
rewrites the manifest. It never fetches content.

## Failure demonstration

The committed [failure evidence](../evidence/golden-failure/README.md) was
created without changing a golden:

```console
cargo xtask golden-demo-failure docs/evidence/golden-failure
```

It flips exactly the center pixel and passes both frames through the same
comparison-artifact path used by `golden-check`.

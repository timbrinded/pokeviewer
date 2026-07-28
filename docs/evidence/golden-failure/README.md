# Visual-golden failure demonstration

This evidence deliberately changes only pixel `(100,100)` in the approved
Monday/Bulbasaur framebuffer. It uses the production comparison-artifact
writer without altering the committed golden.

Regenerate it with:

```console
cargo xtask golden-demo-failure docs/evidence/golden-failure
```

The expected and actual images look almost identical by design. The XOR image
contains the changed center pixel, while `monday-001-report.txt` makes the
one-pixel count, coordinate, and before/after hashes explicit. `golden-check`
uses this comparison path and returns a non-zero status when it finds any such
difference.

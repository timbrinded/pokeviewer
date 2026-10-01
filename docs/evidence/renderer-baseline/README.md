# Shared-renderer baseline evidence

Generated from the committed content pack with:

```console
cargo xtask render-samples docs/evidence/renderer-baseline
```

The PBM and PNG for each record are two deterministic conversions of the same
5,000-byte panel-native framebuffer.

| ID | Card | Frame CRC-32 | PBM SHA-256 | PNG SHA-256 |
| ---: | --- | --- | --- | --- |
| 6 | Charizard, Fire/Flying | `89b5477c` | `4dd5f814a063b7b5cb0dabb9001d779a592971f3163643e0d9a0942523a526cb` | `a710f690be2f7dc6c4eb64f85a2e3f6e90569a005c3ea9339b7f0f4313190d31` |
| 25 | Pikachu, Electric | `735807f7` | `63962f4df08fcb429dc6b159746d4f9963224fe08fc39f55f50a8bc420085a26` | `df5cc0f329c9d9765d3d38213943fadfe43e6260a2f3b31e6391e90635c43495` |
| 29 | Nidoran♀, Poison | `c42b1dbd` | `0458787db8a8370d6d28d9baad475ba33e8ffc924b75dc26665d12dbc008cfe0` | `c0684acc187564215b230ced9e143c3a44f8fe89bbc878cffe7913c3188c390c` |
| 83 | Farfetch’d, Normal/Flying | `b1e091dd` | `133bdf636120958713c8679cbed5c8799c93919cd4faf3d04d630d4c0cb18022` | `bcaefa2e1e51241ba415ac03c350eaf8988a051bc449a0f4ae0b3bd2195005f3` |

All PNG files are 200 × 200, one-bit grayscale, non-interlaced images. The
files contain no child, device, host-path, or USB-identifying data.

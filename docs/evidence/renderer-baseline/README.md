# Shared-renderer baseline evidence

Generated from the committed content pack with:

```console
cargo xtask render-samples docs/evidence/renderer-baseline
```

The PBM and PNG for each record are two deterministic conversions of the same
5,000-byte panel-native framebuffer.

| ID | Card | Frame CRC-32 | PBM SHA-256 | PNG SHA-256 |
| ---: | --- | --- | --- | --- |
| 6 | Charizard, Fire/Flying | `944c4e30` | `8e0c914af8a4cfd280c940932f3d485fb12ec69ce05b138eb15b733b26fe9bfe` | `1ccb59254f69cbcf75fe00277c36f3bdf42c979f743315e154b50cd412ae15ae` |
| 25 | Pikachu, Electric | `2e6e32d6` | `f5b73db88745a6fd537712fad57fbf7faf22698ad64c1ba79264af178ff449ef` | `6bf6dd843e7b9e0f6f5ee4ed445a6fa06c935855bb79456c61bc343f012757de` |
| 29 | Nidoran♀, Poison | `148de2c2` | `131523296c8f48855cfc83e144ce45bfbc976df0a495646b0ad586f6c56b182b` | `4ac870ff3b33016d2fac0da6fa1c1d84a5e79c368a5db3c954b0d59cdbc58679` |
| 83 | Farfetch’d, Normal/Flying | `86be7236` | `690872dc55e7650381577910e4fc53fbdbedd7a4a768ba12badf22d20d412b6a` | `67859db2088411bfc3da6d93b429a41e49ae439ca52dbd0a571fbfb0911c643f` |

All PNG files are 200 × 200, one-bit grayscale, non-interlaced images. The
files contain no child, device, host-path, or USB-identifying data.

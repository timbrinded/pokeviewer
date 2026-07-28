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
| 25 | Pikachu, Electric | `fbdcb388` | `691f39ec7e3c9aa2a247e2907ac12e6e8921eadabe0aae8cfcbf4970094a5e59` | `240c86fbfb13649edb7008e21af9b6836e381cec4cdd8d753981cd3df4e48259` |
| 29 | Nidoran♀, Poison | `148de2c2` | `131523296c8f48855cfc83e144ce45bfbc976df0a495646b0ad586f6c56b182b` | `4ac870ff3b33016d2fac0da6fa1c1d84a5e79c368a5db3c954b0d59cdbc58679` |
| 83 | Farfetch’d, Normal/Flying | `86be7236` | `690872dc55e7650381577910e4fc53fbdbedd7a4a768ba12badf22d20d412b6a` | `67859db2088411bfc3da6d93b429a41e49ae439ca52dbd0a571fbfb0911c643f` |

All PNG files are 200 × 200, one-bit grayscale, non-interlaced images. The
files contain no child, device, host-path, or USB-identifying data.

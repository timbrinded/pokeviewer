# Shared-renderer baseline evidence

Generated from the committed content pack with:

```console
cargo xtask render-samples docs/evidence/renderer-baseline
```

The PBM and PNG for each record are two deterministic conversions of the same
5,000-byte panel-native framebuffer.

| ID | Card | Frame CRC-32 | PBM SHA-256 | PNG SHA-256 |
| ---: | --- | --- | --- | --- |
| 6 | Charizard, Fire/Flying | `29a9fd2a` | `84f7e0649ede808c5f68629d90562d6a7bbdd2efe49578de4e35ba84ea104b4b` | `a41f194ea7a750adbcfa37048c26210f545cdbe63b372b271f951541138e1673` |
| 25 | Pikachu, Electric | `41dbe81e` | `b4d3ca2184e372ecf34bf400af69c85aab37999fb698dc991ad4655dac964457` | `f0f47f5d19dcdc47730ae0930e0cb4ae218c7bcae4c381b2bd7e9f4d020259d3` |
| 29 | Nidoran♀, Poison | `cd889651` | `78405fa84a1619d338e41bebd869c7393e2038fa4f20d7fe94202a7817e2a604` | `d345883c380b24065b54b596e8e1e299ab0b6828bbbaa52c0e327970cce14950` |
| 83 | Farfetch’d, Normal/Flying | `e973e625` | `4189771ac2e9f540b4a311ca0f4b29110a22a069afddd1ceff4aecabcba2607f` | `1985aab89db5d8c5c969cb0e334b56005a2e393542d36caf4ce34ff5df870dd9` |

All PNG files are 200 × 200, one-bit grayscale, non-interlaced images. The
files contain no child, device, host-path, or USB-identifying data.

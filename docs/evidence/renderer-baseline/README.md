# Shared-renderer baseline evidence

Generated from the committed content pack with:

```console
cargo xtask render-samples docs/evidence/renderer-baseline
```

The PBM and PNG for each record are two deterministic conversions of the same
5,000-byte panel-native framebuffer.

| ID | Card | Frame CRC-32 | PBM SHA-256 | PNG SHA-256 |
| ---: | --- | --- | --- | --- |
| 6 | Charizard, Fire/Flying | `eb09da5e` | `f39a572870b862e843566e71b4531ba0d2938908f20b703d7940ca038c03fa15` | `8da050a9162bc36f3656a5c494318389430bda0481554646317ffb908ef18b75` |
| 25 | Pikachu, Electric | `41dbe81e` | `b4d3ca2184e372ecf34bf400af69c85aab37999fb698dc991ad4655dac964457` | `f0f47f5d19dcdc47730ae0930e0cb4ae218c7bcae4c381b2bd7e9f4d020259d3` |
| 29 | Nidoran♀, Poison | `7a305f54` | `c2549d8286e924a22aff0ffa6248cc0a87ef6a2bd8ece238b3558bdd483df50f` | `ba53c1a8d8f7d2f01b95359458bfcb4795d74f36a102e377a917c2eaa194b711` |
| 83 | Farfetch’d, Normal/Flying | `ce8decbd` | `f6afa6691ca11ff68e3aeb8cc9783a15a11db8968174ff623140a75426a078a4` | `03a5bce99299cf6f0fd752337cc01049b1000391010df05db0606978e60ba5d0` |

All PNG files are 200 × 200, one-bit grayscale, non-interlaced images. The
files contain no child, device, host-path, or USB-identifying data.

# Recovery-screen golden evidence

Regenerate all exact one-bit images with:

```console
cargo xtask render-recovery-screens docs/evidence/recovery-screens
```

| Failure | Code | Action | Framebuffer CRC-32 | PNG SHA-256 |
| --- | --- | --- | --- | --- |
| Invalid RTC | `RTC` | wired set/read-back | `34e31d2e` | `7b38717e64138f684cae1119a64530d632de6ed97ffafadc2826c543d575f268` |
| Content pack | `PACK` | reflash verified release | `ee181690` | `1bd7bcab85f3da17816578addb6b64f11a9e09211225f3cc8e0983681ff3440f` |
| Panel | `PANEL` | reset after hardware check | `7086af17` | `1b3ee674728f142b8a6d6b0eb305bd1022575fc58102b7d3c31cdab612ad589c` |
| Alarm arm | `ALARM` | reset after RTC check | `b8ff9d16` | `e58b017b1b998e8eecdb76ba7e133adf85127f77b360ca251b19175ea23d0249` |
| Wake source | `WAKE` | reset | `c9f42a5a` | `18687be64502e365cfa2a577f122db35206400e989fd347997f48e0246c48489` |

All five images are 200 × 200, one-bit grayscale and were visually inspected
at actual pixels on 2026-07-27. A panel failure cannot reliably display its own
screen; its code still has a golden for service and fault-injection checks.
Physical panel photographs remain pending device access.

# V1 daily-card review evidence

`all-cards-contact-sheet.png` contains all 151 exact framebuffers in National
Pokédex order, left to right and then top to bottom. Each card remains exactly
200 × 200 pixels; eight white pixels separate adjacent cards.
Weekdays cycle Monday through Sunday only to exercise every label; the sheet is
a layout audit, not a calendar schedule.

Regenerate it with:

```console
cargo xtask render-contact-sheet \
  docs/evidence/daily-card-v1/all-cards-contact-sheet.png
```

| Property | Value |
| --- | --- |
| Cards | 151 |
| Sheet dimensions | 2,072 × 3,320 pixels |
| PNG format | one-bit grayscale, non-interlaced |
| SHA-256 | `671097e8ae0d3452678e9b67c03d3704b20918017275457c3c60ce3ef800c8cf` |
| Visual inspection | native-pixel desktop review completed 2026-07-27 |
| Physical panel/print review | pending hardware qualification |

The companion `actual-size-review.html` provides four representative cards at
the nominal physical panel size for a 100%-scale print check. Neither artifact
contains child, device, host-path, or USB-identifying data.
